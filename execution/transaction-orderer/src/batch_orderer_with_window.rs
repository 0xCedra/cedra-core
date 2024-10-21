// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    batch_orderer::BatchOrderer,
    common::PTransaction,
    reservation_table::{HashMapReservationTable, ReservationTable},
};
use aptos_types::block_executor::partitioner::TxnIndex;
use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    hash::Hash,
};

/// Returns batches of non-conflicting transactions that additionally do not have dependencies
/// on transactions in recently returned batches. The exact set of transactions that the returned
/// transactions must not depend on can be regulated with the `forget_prefix` method.
pub trait BatchOrdererWithWindow: BatchOrderer {
    /// "Forgets" the `count` first not-yet-forgotten ordered transactions.
    /// When a transaction is forgotten, the orderer no longer guarantees that selected
    /// transactions do not depend on it.
    ///
    /// Each transaction goes through the following stages:
    ///     1. Active: added via `add_transactions`, but not yet returned from `commit_prefix`.
    ///     2. Recently ordered: returned from `commit_prefix`, but not yet forgotten.
    ///        Transactions returned from `commit_prefix` cannot depend on these transactions.
    ///     3. Forgotten: no longer considered by the orderer. Transactions returned from
    ///        `commit_prefix` are allowed to depend on these transactions.
    ///
    /// Note that `self.count_selected()` will not increase unless `forget_prefix` is called.
    ///
    /// `count` must not be greater than `self.get_window_size()`.
    fn forget_prefix(&mut self, count: usize);

    /// Returns the number of not-yet-forgotten ordered transactions.
    fn get_window_size(&self) -> usize;
}

struct TxnInfo<T> {
    transaction: T,
    selected: bool,
    pending_write_table_requests: usize,
    pending_read_table_requests: usize,
    pending_recent_write_dependencies: usize,
}

impl<T> TxnInfo<T> {
    fn can_be_selected(&self) -> bool {
        self.pending_read_table_requests == 0
            && self.pending_write_table_requests == 0
            && self.pending_recent_write_dependencies == 0
    }
}

#[derive(Default)]
struct RecentWriteInfo {
    count: usize,
    dependencies: HashSet<TxnIndex>,
}

pub struct SequentialDynamicWindowOrderer<T: PTransaction> {
    txn_info: Vec<TxnInfo<T>>,
    active_txns_count: usize,

    selected: BTreeSet<TxnIndex>,

    write_reservations: HashMapReservationTable<T::Key, TxnIndex>,
    read_reservations: HashMapReservationTable<T::Key, TxnIndex>,

    recently_committed_txns: VecDeque<TxnIndex>,
    recent_writes: HashMap<T::Key, RecentWriteInfo>,
}

impl<T: PTransaction> Default for SequentialDynamicWindowOrderer<T> {
    // NB: unfortunately, Rust cannot derive Default for generic structs
    // with type parameters that do not implement Default.
    // See: https://github.com/rust-lang/rust/issues/26925
    fn default() -> Self {
        Self {
            txn_info: Default::default(),
            active_txns_count: Default::default(),

            selected: Default::default(),

            write_reservations: Default::default(),
            read_reservations: Default::default(),

            recently_committed_txns: Default::default(),
            recent_writes: Default::default(),
        }
    }
}

impl<T> SequentialDynamicWindowOrderer<T>
where
    T: PTransaction + Clone,
    T::Key: Hash + Eq + Clone,
{
    fn satisfy_pending_read_table_request(&mut self, idx: TxnIndex) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_read_table_requests >= 1);
        tx_info.pending_read_table_requests -= 1;

        assert!(!tx_info.selected);
        if tx_info.can_be_selected() {
            tx_info.selected = true;
            self.selected.insert(idx);
        }
    }

    fn satisfy_pending_write_table_request(&mut self, idx: TxnIndex, key: &T::Key) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_write_table_requests >= 1);
        tx_info.pending_write_table_requests -= 1;

        // Register the dependency on the recent write.
        // This transaction cannot be committed until this dependency is satisfied.
        if self
            .recent_writes
            .get_mut(key)
            .unwrap()
            .dependencies
            .insert(idx)
        {
            tx_info.pending_recent_write_dependencies += 1;
        }
    }

    fn resolve_recent_write_dependency(&mut self, idx: TxnIndex) {
        let tx_info = &mut self.txn_info[idx];
        assert!(tx_info.pending_recent_write_dependencies >= 1);
        tx_info.pending_recent_write_dependencies -= 1;

        assert!(!tx_info.selected);
        if tx_info.can_be_selected() {
            tx_info.selected = true;
            self.selected.insert(idx);
        }
    }
}

impl<T> BatchOrderer for SequentialDynamicWindowOrderer<T>
where
    T: PTransaction + Clone,
    T::Key: Hash + Eq + Clone,
{
    type Txn = T;

    fn add_transactions<TS>(&mut self, txns: TS)
    where
        TS: IntoIterator<Item = Self::Txn>,
    {
        for tx in txns {
            let idx = self.txn_info.len();
            self.active_txns_count += 1;

            self.write_reservations
                .make_reservations(idx, tx.write_set());
            self.read_reservations.make_reservations(idx, tx.read_set());

            let mut pending_recent_write_dependencies = 0;
            if !self.recent_writes.is_empty() {
                for k in tx.write_set() {
                    if let Some(write_info) = self.recent_writes.get_mut(k) {
                        pending_recent_write_dependencies += 1;
                        write_info.dependencies.insert(idx);
                    }
                }
            }

            let pending_write_table_requests =
                self.write_reservations.make_requests(idx, tx.read_set());
            let pending_read_table_requests =
                self.read_reservations.make_requests(idx, tx.write_set());

            let selected = pending_recent_write_dependencies == 0
                && pending_write_table_requests == 0
                && pending_read_table_requests == 0;

            if selected {
                //println!("Selected txn id {}", tx.get_id());
                self.selected.insert(idx);
            } else {
                //println!("Not selected txn id {}; pending_recent_write_dependencies {}; pending_write_table_requests {}; pending_read_table_requests {}",
                  //       tx.get_id(), pending_recent_write_dependencies, pending_write_table_requests, pending_read_table_requests);
            }

            self.txn_info.push(TxnInfo {
                transaction: tx,
                selected,
                pending_write_table_requests,
                pending_read_table_requests,
                pending_recent_write_dependencies,
            });
        }
    }

    fn count_active_transactions(&self) -> usize {
        self.active_txns_count
    }

    fn count_selected(&self) -> usize {
        self.selected.len()
    }

    fn commit_prefix_callback<F, R>(&mut self, count: usize, callback: F) -> R
    where
        F: FnOnce(Vec<Self::Txn>) -> R,
    {
        assert!(count <= self.count_selected());

        let committed_indices: Vec<_> = (0..count)
            .map(|_| self.selected.pop_first().unwrap())
            .collect();

        let committed_txns: Vec<_> = committed_indices
            .iter()
            .map(|&idx| self.txn_info[idx].transaction.clone())
            .collect();

        // Return the committed transactions early via the callback, to minimize latency.
        // Note that the callback cannot access the Orderer as we are still holding a mutable
        // reference to it. Hence, it will not be able to observe the orderer in an inconsistent
        // state.
        for tx in committed_txns.iter() {
        //    println!("Ordered txn id {}", tx.get_id());
        }
        let res = callback(committed_txns);

        // Update the internal data structures.
        self.active_txns_count -= count;

        for &committed_idx in committed_indices.iter() {
            let tx = &self.txn_info[committed_idx].transaction;

            self.recently_committed_txns.push_back(committed_idx);
            for key in tx.write_set() {
                self.recent_writes.entry(key.clone()).or_default().count += 1;
            }

            let satisfied_write_table_requests = self
                .write_reservations
                .remove_reservations(committed_idx, tx.write_set());

            for (idx, key) in satisfied_write_table_requests {
                self.satisfy_pending_write_table_request(idx, &key);
            }
        }

        // The read table requests have to be processed after all the write table requests,
        // are processed to make sure that `pending_recent_write_dependencies` is updated.
        for &committed_idx in committed_indices.iter() {
            let tx = &self.txn_info[committed_idx].transaction;

            let satisfied_read_table_requests = self
                .read_reservations
                .remove_reservations(committed_idx, tx.read_set());

            for (idx, _) in satisfied_read_table_requests {
                self.satisfy_pending_read_table_request(idx);
            }
        }

        res
    }
}

impl<T> BatchOrdererWithWindow for SequentialDynamicWindowOrderer<T>
where
    T: PTransaction + Clone,
    T::Key: Hash + Eq + Clone,
{
    fn forget_prefix(&mut self, count: usize) {
        assert!(count <= self.get_window_size());
        let forgotten_indices = self
            .recently_committed_txns
            .drain(0..count)
            .collect::<Vec<_>>();

        for forgotten_idx in forgotten_indices {
            let tx = &self.txn_info[forgotten_idx].transaction;
            let write_set: Vec<_> = tx.write_set().cloned().collect();
            for key in write_set {
                let write_info = self.recent_writes.get_mut(&key).unwrap();
                write_info.count -= 1;
                if write_info.count == 0 {
                    let write_info = self.recent_writes.remove(&key).unwrap();
                    for &idx in write_info.dependencies.iter() {
                        self.resolve_recent_write_dependency(idx);
                    }
                }
            }
        }
    }

    fn get_window_size(&self) -> usize {
        self.recently_committed_txns.len()
    }
}
