// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a processor that determines which writes to temporaries
//! are better "flushed" immediately by the file format code generator. Read on
//! for more information on what "flushing" means.
//!
//! For this pass to be effective, it should be run after all the stackless-bytecode
//! transformations are done, because the annotations produced by it are used
//! (when available) by the file-format generator. Code transformations render
//! previously computed annotations invalid.
//!
//! A pre-requisite for this pass is the live-variable analysis annotations.
//!
//! The file format generator can keep some writes to temporaries only on the stack,
//! not writing it back to local memory (as a potential optimization).
//! However, this is not always good, and this pass helps determine when a write to
//! a temporary should be flushed right away.
//! In the context of file format code generator, "flushed" means either store the
//! value to a local (if used later) or pop if from the stack (if not used later).
//! Currently, we instruct to flush those temps right away that are not used within
//! the same basic block.

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

/// For a given code offset, tracks which temporaries written at the code offset
/// should be flushed right away by the file format generator.
#[derive(Clone)]
pub struct FlushWritesAnnotation(pub BTreeMap<CodeOffset, BTreeSet<TempIndex>>);

/// A processor for computing the `FlushWritesAnnotation`.
pub struct FlushWritesProcessor {}

impl FunctionTargetProcessor for FlushWritesProcessor {
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
        let live_vars = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation is a prerequisite");
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let mut unused: BTreeMap<CodeOffset, BTreeSet<TempIndex>> = BTreeMap::new();
        for block_id in cfg.blocks() {
            if let Some((lower, upper)) = cfg.instr_offset_bounds(block_id) {
                extract_unused_writes_in_block(lower, upper, code, live_vars, &mut unused);
            }
        }
        data.annotations.set(FlushWritesAnnotation(unused), true);
        data
    }

    fn name(&self) -> String {
        "FlushWritesProcessor".to_string()
    }
}

/// In the basic block defined by `code[lower..=upper]`, extract the writes to
/// temporaries that are not used later the block. At the offset where the write
/// happens, such temporaries are included, in `unused`.
fn extract_unused_writes_in_block(
    lower: u16,
    upper: u16,
    code: &[Bytecode],
    live_vars: &LiveVarAnnotation,
    unused: &mut BTreeMap<CodeOffset, BTreeSet<TempIndex>>,
) {
    for offset in lower..=upper {
        let instr = &code[offset as usize];
        // Only `Load` and `Call` instructions push results to the stack.
        if matches!(instr, Bytecode::Load(..) | Bytecode::Call(..)) {
            if let Some(live_info) = live_vars.get_live_var_info_at(offset) {
                for dest in instr.dests() {
                    if let Some(info) = live_info.after.get(&dest) {
                        // Note: loop-carried uses are not considered here.
                        let all_usages_are_outside_block = info
                            .usage_offsets()
                            .iter()
                            .all(|usage| *usage <= offset || *usage > upper);
                        if all_usages_are_outside_block {
                            unused.entry(offset).or_default().insert(dest);
                        }
                    } else {
                        // `dest` is not alive after `offset`, so it is not used.
                        unused.entry(offset).or_default().insert(dest);
                    }
                }
            }
        }
    }
}

impl FlushWritesProcessor {
    /// Registers annotation formatter at the given function target.
    /// Helps with testing and debugging.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_flush_writes_annotation));
    }
}

// ====================================================================
// Formatting functionality for flush writes annotation

pub fn format_flush_writes_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let FlushWritesAnnotation(map) = target.get_annotations().get::<FlushWritesAnnotation>()?;
    let temps = map.get(&code_offset)?;
    if temps.is_empty() {
        return None;
    }
    let mut res = "flush: ".to_string();
    res.push_str(
        &temps
            .iter()
            .map(|t| {
                let name = target.get_local_raw_name(*t);
                format!("{}", name.display(target.symbol_pool()))
            })
            .join(", "),
    );
    Some(res)
}
