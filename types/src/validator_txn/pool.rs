// Copyright © Aptos Foundation

use crate::validator_txn::ValidatorTransaction;
use aptos_crypto::HashValue;
use std::collections::HashSet;

pub enum ValidatorTransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
}

pub trait ValidatorTransactionPoolClient: Send + Sync {
    fn pull(
        &self,
        max_items: u64,
        max_bytes: u64,
        exclude: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction>;
}

pub struct ValidatorTransactionPool {}

impl ValidatorTransactionPool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ValidatorTransactionPool {
    fn default() -> Self {
        Self::new()
    }
}
impl ValidatorTransactionPoolClient for ValidatorTransactionPool {
    fn pull(
        &self,
        _max_items: u64,
        _max_bytes: u64,
        _exclude: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        vec![]
    }
}
