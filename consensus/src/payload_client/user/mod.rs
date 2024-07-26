// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::PayloadPullParameters;
use crate::error::QuorumStoreError;
use aptos_consensus_types::common::Payload;
#[cfg(test)]
use aptos_types::transaction::SignedTransaction;
use futures::future::BoxFuture;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use std::time::Instant;

/// Clients can pull information about transactions from the mempool and return
/// the retrieved information as a `Payload`.
#[async_trait::async_trait]
pub trait UserPayloadClient: Send + Sync {
    async fn pull(
        &self,
        params: PayloadPullParameters,
        wait_callback: BoxFuture<'static, ()>,
    ) -> anyhow::Result<Payload, QuorumStoreError>;
}

/// A dummy user payload client that pull hardcoded txns one by one.
#[cfg(test)]
pub struct DummyClient {
    pub(crate) txns: Vec<SignedTransaction>,
}

#[cfg(test)]
impl DummyClient {
    pub fn new(txns: Vec<SignedTransaction>) -> Self {
        Self { txns }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl UserPayloadClient for DummyClient {
    async fn pull(
        &self,
        mut params: PayloadPullParameters,
        wait_callback: BoxFuture<'static, ()>,
    ) -> anyhow::Result<Payload, QuorumStoreError> {
        let timer = Instant::now();
        let mut nxt_txn_idx = 0;
        let mut txns = vec![];
        while timer.elapsed() < params.max_poll_time
            && params.max_txns.count >= 1
            && params.max_txns_after_filtering >= 1
            && params.soft_max_txnss_after_filtering >= 1
            && params.max_txns.bytes >= 1
            && nxt_txn_idx < self.txns.len()
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
            let txn = self.txns[nxt_txn_idx].clone();
            let txn_size = txn.raw_txn_bytes_len() as u64;
            if txn_size > params.max_txns.bytes {
                break;
            }
            params.max_txns.count -= 1;
            params.max_unique_txns -= 1;
            params.soft_max_items_after_filtering -= 1;
            params.max_txns.bytes -= txn_size;
            nxt_txn_idx += 1;
            txns.push(txn);
        }
        Ok(Payload::DirectMempool(txns))
    }
}

pub mod quorum_store_client;
