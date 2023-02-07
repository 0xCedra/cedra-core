// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{error::QuorumStoreError, monitor, state_replication::PayloadClient};
use anyhow::Result;
use aptos_consensus_types::{
    common::{Payload, PayloadFilter, Round},
    request_response::{BlockProposalCommand, ConsensusResponse},
};
use aptos_logger::prelude::*;
use fail::fail_point;
use futures::{
    channel::{mpsc, oneshot},
    future::BoxFuture,
};
use std::time::Duration;
use tokio::time::{sleep, timeout};

const NO_TXN_DELAY: u64 = 30; // TODO: consider moving to a config

/// Client that pulls blocks from Quorum Store
#[derive(Clone)]
pub struct QuorumStoreClient {
    consensus_to_quorum_store_sender: mpsc::Sender<BlockProposalCommand>,
    poll_count: u64,
    /// Timeout for consensus to pull transactions from quorum store and get a response (in milliseconds)
    pull_timeout_ms: u64,
    create_empty_blocks_for_pending_ordering: bool,
    create_partial_blocks_before_poll_ends: bool,
}

impl QuorumStoreClient {
    pub fn new(
        consensus_to_quorum_store_sender: mpsc::Sender<BlockProposalCommand>,
        poll_count: u64,
        pull_timeout_ms: u64,
        create_empty_blocks_for_pending_ordering: bool,
        create_partial_blocks_before_poll_ends: bool,
    ) -> Self {
        assert!(
            poll_count > 0,
            "poll_count = 0 won't pull any txns from quorum store"
        );
        Self {
            consensus_to_quorum_store_sender,
            poll_count,
            pull_timeout_ms,
            create_empty_blocks_for_pending_ordering,
            create_partial_blocks_before_poll_ends,
        }
    }

    async fn pull_internal(
        &self,
        round: Round,
        max_items: u64,
        max_bytes: u64,
        return_non_full: bool,
        exclude_payloads: PayloadFilter,
    ) -> Result<Payload, QuorumStoreError> {
        let (callback, callback_rcv) = oneshot::channel();
        let req = BlockProposalCommand::GetBlockRequest(
            round,
            max_items,
            max_bytes,
            return_non_full,
            exclude_payloads.clone(),
            callback,
        );
        // send to shared mempool
        self.consensus_to_quorum_store_sender
            .clone()
            .try_send(req)
            .map_err(anyhow::Error::from)?;
        // wait for response
        match monitor!(
            "pull_payload",
            timeout(Duration::from_millis(self.pull_timeout_ms), callback_rcv).await
        ) {
            Err(_) => {
                Err(anyhow::anyhow!("[consensus] did not receive GetBlockResponse on time").into())
            },
            Ok(resp) => match resp.map_err(anyhow::Error::from)?? {
                ConsensusResponse::GetBlockResponse(payload) => Ok(payload),
            },
        }
    }
}

#[async_trait::async_trait]
impl PayloadClient for QuorumStoreClient {
    async fn pull_payload(
        &self,
        round: Round,
        max_items: u64,
        max_bytes: u64,
        exclude_payloads: PayloadFilter,
        wait_callback: BoxFuture<'static, ()>,
        pending_ordering: bool,
    ) -> Result<Payload, QuorumStoreError> {
        let return_empty = pending_ordering && self.create_empty_blocks_for_pending_ordering;
        let return_non_full = self.create_partial_blocks_before_poll_ends;

        fail_point!("consensus::pull_payload", |_| {
            Err(anyhow::anyhow!("Injected error in pull_payload").into())
        });
        let mut callback_wrapper = Some(wait_callback);
        // keep polling QuorumStore until there's payloads available or there's still pending payloads
        let mut count = self.poll_count;
        let payload = loop {
            count -= 1;
            let payload = self
                .pull_internal(
                    round,
                    max_items,
                    max_bytes,
                    return_non_full || return_empty || count == 0 || self.poll_count == u64::MAX,
                    exclude_payloads.clone(),
                )
                .await?;
            if payload.is_empty() && !return_empty && count > 0 {
                if let Some(callback) = callback_wrapper.take() {
                    callback.await;
                }
                sleep(Duration::from_millis(NO_TXN_DELAY)).await;
                continue;
            }
            break payload;
        };
        info!(
            poll_count = self.poll_count - count,
            max_poll_count = self.poll_count,
            payload_len = payload.len(),
            max_items = max_items,
            max_bytes = max_bytes,
            pending_ordering = pending_ordering,
            return_empty = return_empty,
            return_non_full = return_non_full,
            "Pull payloads from QuorumStore: proposal"
        );
        Ok(payload)
    }
}
