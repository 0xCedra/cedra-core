// Copyright © Aptos Foundation

use std::collections::HashMap;
use std::fmt::Debug;
use std::slice::Iter;
use std::sync::Arc;
use aptos_mvhashmap::MVHashMap;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::executable::Executable;
use crate::scheduler::Scheduler;
use crate::task::{Transaction, TransactionOutput};
use crate::txn_last_input_output::TxnOutput;
use crate::txn_provider::{TxnProviderTrait1, TxnProviderTrait2};

/// Some logic of vanilla BlockSTM.
pub struct DefaultTxnProvider<T> {
    txns: Vec<T>,
}

impl<T> DefaultTxnProvider<T> {
    pub fn new(txns: Vec<T>) -> Self {
        Self {
            txns
        }
    }
}

impl<T> TxnProviderTrait1 for DefaultTxnProvider<T> {
    fn end_txn_idx(&self) -> TxnIndex {
        self.txns.len() as TxnIndex
    }

    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn first_txn(&self) -> TxnIndex {
        if self.num_txns() == 0 { self.end_txn_idx() } else { 0 }
    }

    fn next_txn(&self, idx: TxnIndex) -> TxnIndex {
        if idx == self.end_txn_idx() { idx } else { idx + 1 }
    }

    fn txns(&self) -> Vec<TxnIndex> {
        (0..self.num_txns() as TxnIndex).collect()
    }

    fn txns_and_deps(&self) -> Vec<TxnIndex> {
        self.txns()
    }

    fn local_rank(&self, idx: TxnIndex) -> usize {
        idx as usize
    }

    fn is_local(&self, _idx: TxnIndex) -> bool {
        true
    }

    fn txn_output_has_arrived(&self, txn_idx: TxnIndex) -> bool {
        unreachable!()
    }

    fn block_idx(&self) -> u8 {
        0
    }

    fn shard_idx(&self) -> usize {
        0
    }
}

impl<T, TO, TE> TxnProviderTrait2<T, TO, TE> for DefaultTxnProvider<T>
where
    T: Transaction,
    TO: TransactionOutput<Txn = T>,
    TE: Debug + Send + Clone,
{
    fn remote_dependencies(&self) -> Vec<(TxnIndex, T::Key)> {
        vec![]
    }

    fn run_sharding_msg_loop<X: Executable + 'static>(&self, mv_cache: &MVHashMap<T::Key, T::Value, X>, scheduler: &Scheduler<Self>) {
        // Nothing to do.
    }

    fn shutdown_receiver(&self) {
        // Nothing to do.
    }

    fn txn(&self, idx: TxnIndex) -> &T {
        &self.txns[idx as usize]
    }

    fn on_local_commit(&self, _txn_idx: TxnIndex, _txn_output: Arc<TxnOutput<TO, TE>>) {
        // Nothing to do.
    }

    fn commit_strategy(&self) -> u8 {
        0
    }
}
