// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a transformation that reuses locals of the same type when
//! possible.
//! prerequisite: livevar annotation is available by performing liveness analysis.
//! side effect: this transformation removes all pre-existing annotations.
//!
//! This transformation is closely related to the register allocation problem in
//! compilers. As such, an optimal solution to reusing locals is NP-complete, as we
//! can show its equivalence to the graph coloring problem.
//!
//! Our solution here is inspired by the paper "Linear Scan Register Allocation"
//! by Poletto and Sarkar, which proposes a fast and greedy register allocation
//! algorithm. While potentially suboptimal, it is simple and fast, and is known to
//! produce good results in practice. Our solution uses some key ideas from that
//! paper, and performs a linear scan for deciding which locals to reuse.
//!
//! A key concept in this transformation is the "live interval" of a local, as opposed
//! to the more precise "live range" (the set of code offsets where a local is live).
//! The live interval of a local `t` is a consecutive range of code offsets `[i, j]`
//! such that there is no code offset `j' > j` where `t` is live at `j'`, and there is
//! no code offset `i' < i` where `t` is live at `i'`. A trivial live interval for any
//! local is `[0, MAX_CODE_OFFSET]`, but we can often compute more precise live intervals.
//!
//! The transformation greedily reuses (same-typed) locals outside their live intervals.

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv, ty::Type};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
};
use std::collections::{BTreeMap, BTreeSet};

/// The live interval of a local (inclusive).
/// Note that two live intervals i1: [b1, x] and i2: [x, e2] are *not* considered to overlap
/// even though the code offset `x` is in both intervals.
struct LiveInterval {
    begin: CodeOffset,
    end: CodeOffset,
}

impl LiveInterval {
    /// Create a new live interval that only has the given offset.
    fn new(offset: CodeOffset) -> Self {
        Self {
            begin: offset,
            end: offset,
        }
    }

    /// Include the given offset in the live interval, expanding the interval as necessary.
    fn include(&mut self, offset: CodeOffset) {
        self.begin = std::cmp::min(self.begin, offset);
        self.end = std::cmp::max(self.end, offset);
    }
}

/// Live interval event of a local, used for sorting.
enum LiveIntervalEvent {
    Begin(
        /* which local? */ TempIndex,
        /* live interval begins */ CodeOffset,
        /* live interval length */ usize, // used for tie-breaking
    ),
    End(
        /* which local? */ TempIndex,
        /* live interval ends */ CodeOffset,
    ),
}

impl LiveIntervalEvent {
    /// Get the code offset at which the event occurs.
    fn offset(&self) -> CodeOffset {
        match self {
            LiveIntervalEvent::Begin(_, offset, _) => *offset,
            LiveIntervalEvent::End(_, offset) => *offset,
        }
    }
}

pub struct VariableCoalescing {}

impl VariableCoalescing {
    /// Compute the live intervals of locals in the given function target.
    /// The result is a vector of live intervals, where the index of the vector is the local.
    /// If a local has `None` as its live interval, we can ignore the local for the coalescing
    /// transformation (eg., because it is borrowed or because it is never used): it implies
    /// that it is the trivial live interval.
    fn live_intervals(target: &FunctionTarget) -> Vec<Option<LiveInterval>> {
        let LiveVarAnnotation(live_var_infos) = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live var annotation is a prerequisite");
        // Note: we currently exclude all the variables that are borrowed from participating in this
        // transformation, which is safe. However, we could be more precise in this regard.
        let borrowed_locals = target.get_borrowed_locals();
        // Initially, all locals have trivial live intervals.
        let mut live_intervals = std::iter::repeat_with(|| None)
            .take(target.get_local_count())
            .collect::<Vec<_>>();
        for (offset, live_var_info) in live_var_infos.iter() {
            live_var_info
                .after
                .keys()
                .chain(live_var_info.before.keys())
                .filter(|local| !borrowed_locals.contains(local))
                .for_each(|local| {
                    // non-borrowed local that is live before and/or after the code offset.
                    let interval =
                        live_intervals[*local].get_or_insert_with(|| LiveInterval::new(*offset));
                    interval.include(*offset);
                });
        }
        live_intervals
    }

    /// Compute the sorted live interval events of locals in the given function target.
    /// See implementation comments for the sorting order.
    fn sorted_live_interval_events(target: &FunctionTarget) -> Vec<LiveIntervalEvent> {
        let live_intervals = Self::live_intervals(target);
        let mut live_interval_events = vec![];
        for (local, interval) in live_intervals.into_iter().enumerate() {
            if let Some(interval) = interval {
                live_interval_events.push(LiveIntervalEvent::Begin(
                    local as TempIndex,
                    interval.begin,
                    (interval.end - interval.begin) as usize,
                ));
                live_interval_events.push(LiveIntervalEvent::End(local as TempIndex, interval.end));
            }
        }
        live_interval_events.sort_by(|a, b| {
            use LiveIntervalEvent::*;
            match (a, b) {
                // Sort events based on their code offsets (lower offset comes first).
                _ if a.offset() < b.offset() => std::cmp::Ordering::Less,
                _ if a.offset() > b.offset() => std::cmp::Ordering::Greater,
                // If two events occur at the same offset, `End` comes before `Before`.
                // This allows locals in `End` to be possibly remapped to locals in `Begin`
                // at the same code offset.
                (End(..), Begin(..)) => std::cmp::Ordering::Less,
                (Begin(..), End(..)) => std::cmp::Ordering::Greater,
                // If two locals `End` at the same offset, then we arbitrarily (but deterministically)
                // use the local index to break the tie.
                (End(local_a, _), End(local_b, _)) => local_a.cmp(local_b),
                // If two locals `Begin` at the same offset, then we use the length of their live
                // intervals to break the tie. The idea behind this heuristic is that remapping a
                // local with shorter interval might make it available for remapping to other locals
                // sooner. If the intervals are of the same length, we arbitrarily (but deterministically)
                // use the local index to break the tie.
                (Begin(local_a, _, length_a), Begin(local_b, _, length_b)) => {
                    length_a.cmp(length_b).then_with(|| local_a.cmp(local_b))
                },
            }
        });
        live_interval_events
    }

    /// Compute the coalesceable locals of the given function target.
    /// The result is a map, where for each mapping from local `t` to its coalesceable local `u`,
    /// we can safely replace all occurrences of `t` with `u`.
    /// This safety property follows from:
    ///   - `t` and `u` are of the same type
    ///   - either the live intervals of `t` and `u` do not overlap, in which case, they do not interfere
    ///     with each others computations,
    ///   - or `t` becomes live for the first time at the same code offset as `u` is last seen alive
    ///     (i.e., `u` is one of the sources and `t` is one of the destinations at the code offset), in
    ///     which case, we can safely reuse `u` in place of `t`.
    fn coalesceable_locals(target: &FunctionTarget) -> BTreeMap<TempIndex, TempIndex> {
        let sorted_events = Self::sorted_live_interval_events(target);
        // Map local `t` to its coalesceable local `u`, where the replacement `t` -> `u` is safe.
        let mut coalesceable_locals = BTreeMap::new();
        // For each type in the function, keep track of the available locals (not alive) of that type.
        let mut avail_map: BTreeMap<&Type, BTreeSet<TempIndex>> = BTreeMap::new();
        for event in sorted_events {
            match event {
                LiveIntervalEvent::Begin(local, _, _) => {
                    let local_type = target.get_local_type(local);
                    if let Some(avail_locals) = avail_map.get_mut(local_type) {
                        if let Some(avail) = avail_locals.pop_first() {
                            // We found a local `avail` that is not alive with matching types.
                            // Let's use it to replace occurrences of `local`.
                            coalesceable_locals.insert(local, avail);
                        }
                    }
                },
                LiveIntervalEvent::End(local, _) => {
                    let local_type = target.get_local_type(local);
                    let avail_local = *coalesceable_locals.get(&local).unwrap_or(&local);
                    // `local` is no longer alive, so it can be reused.
                    avail_map.entry(local_type).or_default().insert(avail_local);
                },
            }
        }
        coalesceable_locals
    }

    /// Obtain the transformed code of the given function target by reusing coalesceable locals.
    /// The resulting code can potentially leave several locals unused.
    fn transform(target: &FunctionTarget) -> Vec<Bytecode> {
        let coalesceable_locals = Self::coalesceable_locals(target);
        let mut new_code = vec![];
        let mut remapping_locals =
            |local: TempIndex| *coalesceable_locals.get(&local).unwrap_or(&local);
        for instr in target.get_bytecode() {
            let remapped_instr = instr.clone().remap_all_vars(target, &mut remapping_locals);
            new_code.push(remapped_instr);
        }
        new_code
    }
}

impl FunctionTargetProcessor for VariableCoalescing {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        data.code = Self::transform(&target);
        // Annotations may no longer be valid after this transformation.
        // So remove them.
        data.annotations.clear();
        data
    }

    fn name(&self) -> String {
        "VariableCoalescing".to_string()
    }
}
