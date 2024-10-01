// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod blocking_txns_provider;
pub mod default;

use crate::transaction::BlockExecutableTransaction as Transaction;
use std::sync::Arc;

pub type TxnIndex = u32;

pub trait TxnProvider<T: Transaction> {
    /// Get total number of transactions
    fn num_txns(&self) -> usize;

    /// Get a reference of the txn object by its index.
    fn get_txn(&self, idx: TxnIndex) -> Arc<T>;
}
