// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{batch_reader::BatchReader, utils::RoundExpirations};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::debug;
use aptos_types::transaction::SignedTransaction;
use arc_swap::ArcSwapOption;
use consensus_types::{
    block::Block,
    common::Payload,
    proof_of_store::{LogicalTime, ProofOfStore},
    request_response::WrapperCommand,
};
use dashmap::DashMap;
use executor_types::*;
use futures::channel::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Notification of execution committed logical time for QuorumStore to clean.
#[async_trait::async_trait]
pub trait DataManager: Send + Sync {
    /// Notification of committed logical time
    async fn notify_commit(&self, logical_time: LogicalTime, payloads: Vec<Payload>);

    fn new_epoch(
        &self,
        data_reader: Arc<BatchReader>,
        quorum_store_wrapper_tx: Sender<WrapperCommand>,
    );

    async fn update_payload(&self, block: &Block);

    async fn get_data(&self, block: &Block) -> Result<Vec<SignedTransaction>, Error>;
}

enum DataStatus {
    Cached(Vec<SignedTransaction>),
    Requested(Vec<oneshot::Receiver<Result<Vec<SignedTransaction>, Error>>>),
}

/// Execution -> QuorumStore notification of commits.
pub struct QuorumStoreDataManager {
    data_reader: ArcSwapOption<BatchReader>,
    quorum_store_wrapper_tx: ArcSwapOption<Sender<WrapperCommand>>,
    digest_status: DashMap<HashValue, DataStatus>,
    expiration_status: Mutex<RoundExpirations<HashValue>>,
}

impl QuorumStoreDataManager {
    /// new
    pub fn new() -> Self {
        Self {
            data_reader: ArcSwapOption::from(None),
            quorum_store_wrapper_tx: ArcSwapOption::from(None),
            digest_status: DashMap::new(),
            expiration_status: Mutex::new(RoundExpirations::new()),
        }
    }
}

impl QuorumStoreDataManager {
    async fn request_data(
        &self,
        poss: Vec<ProofOfStore>,
        logical_time: LogicalTime,
    ) -> Vec<oneshot::Receiver<Result<Vec<SignedTransaction>, executor_types::Error>>> {
        let mut receivers = Vec::new();
        for pos in poss {
            debug!(
                "QSE: requesting pos {:?}, digest {}, time = {:?}",
                pos,
                pos.digest(),
                logical_time
            );
            if logical_time <= pos.expiration() {
                receivers.push(
                    self.data_reader
                        .load()
                        .as_ref()
                        .unwrap() //TODO: can this be None? Need to make sure we call new_epoch() first.
                        .get_batch(pos)
                        .await,
                );
            } else {
                debug!("QS: skipped expired pos");
            }
        }
        receivers
    }
}

#[async_trait::async_trait]
impl DataManager for QuorumStoreDataManager {
    // Execution result has been certified (TODO: double check).
    async fn notify_commit(&self, logical_time: LogicalTime, payloads: Vec<Payload>) {
        self.data_reader
            .load()
            .as_ref()
            .unwrap()
            .update_certified_round(logical_time)
            .await;

        let payload_is_empty = payloads.is_empty();

        let digests: Vec<HashValue> = payloads
            .into_iter()
            .map(|payload| match payload {
                Payload::DirectMempool(_) => {
                    unreachable!()
                }
                Payload::InQuorumStore(proofs) => proofs,
                Payload::Empty => Vec::new(),
            })
            .flatten()
            .map(|proof| proof.digest().clone())
            .collect();

        let _ = self
            .quorum_store_wrapper_tx
            .load()
            .as_ref()
            .unwrap()
            .as_ref()
            .clone()
            .try_send(WrapperCommand::CleanRequest(logical_time, digests));

        if !payload_is_empty {
            let expired_set = self.expiration_status.lock().expire(logical_time.round());
            for expired in expired_set {
                self.digest_status.remove(&expired);
            }
        }
    }

    async fn update_payload(&self, block: &Block) {
        if block.payload().is_some() {
            match block.payload().unwrap() {
                Payload::InQuorumStore(proofs) => {
                    if !self.digest_status.contains_key(&block.id()) {
                        let receivers = self
                            .request_data(
                                proofs.clone(),
                                LogicalTime::new(block.epoch(), block.round()),
                            )
                            .await;
                        self.digest_status
                            .insert(block.id(), DataStatus::Requested(receivers));
                        self.expiration_status
                            .lock()
                            .add_item(block.id(), block.round());
                    }
                }
                Payload::Empty => {}
                Payload::DirectMempool(_) => {
                    unreachable!()
                }
            }
        }
    }

    async fn get_data(&self, block: &Block) -> Result<Vec<SignedTransaction>, Error> {
        if block.payload().is_none() {
            return Ok(Vec::new());
        }
        match block.payload().unwrap() {
            Payload::Empty => {
                debug!("QSE: empty Payload");
                Ok(Vec::new())
            }
            Payload::DirectMempool(_) => unreachable!("Direct mempool should not be used."),
            Payload::InQuorumStore(proofs) => {
                // let data_status = self.digest_status.entry(block.id());
                match self.digest_status.entry(block.id()) {
                    dashmap::mapref::entry::Entry::Occupied(mut entry) => match entry.get_mut() {
                        DataStatus::Cached(data) => {
                            return Ok(data.clone());
                        }
                        DataStatus::Requested(receivers) => {
                            let mut vec_ret = Vec::new();
                            debug!("QSE: waiting for data on {} receivers", receivers.len());
                            for rx in receivers {
                                match rx.await {
                                    Err(_) => {
                                        // We probably advanced epoch already.
                                        warn!("Oneshot channel to get a batch was dropped");
                                        let new_receivers = self
                                            .request_data(
                                                proof_with_status.proofs.clone(),
                                                LogicalTime::new(block.epoch(), block.round()),
                                            )
                                            .await;
                                        // Could not get all data so requested again
                                        proof_with_status
                                            .status
                                            .lock()
                                            .replace(DataStatus::Requested(new_receivers));
                                        return Err(BlockNotFound(block.id()));
                                    }
                                    Ok(result) => match result {
                                        Ok(data) => {
                                            debug!("QSE: got data, len {}", data.len());
                                            vec_ret.push(data);
                                        }
                                        Err(e) => {
                                            debug!("QS: got error from receiver {:?}", e);
                                            let new_receivers = self
                                                .request_data(
                                                    proofs.clone(),
                                                    LogicalTime::new(block.epoch(), block.round()),
                                                )
                                                .await;
                                            entry.replace_entry(DataStatus::Requested(
                                                new_receivers,
                                            ));
                                            return Err(e);
                                        }
                                    },
                                }
                            }
                            let ret: Vec<SignedTransaction> =
                                vec_ret.into_iter().flatten().collect();
                            entry.replace_entry(DataStatus::Cached(ret.clone()));

                            Ok(ret)
                        }
                    },
                    dashmap::mapref::entry::Entry::Vacant(_) => {
                        unreachable!("digest_status entry must exist!");
                    }
                }
            }
        }
    }

    fn new_epoch(
        &self,
        data_reader: Arc<BatchReader>,
        quorum_store_wrapper_tx: Sender<WrapperCommand>,
    ) {
        // TODO: check race here.
        self.data_reader.swap(Some(data_reader));
        self.quorum_store_wrapper_tx
            .swap(Some(Arc::from(quorum_store_wrapper_tx)));
    }
}

pub struct DummyDataManager {}

impl DummyDataManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl DataManager for DummyDataManager {
    async fn notify_commit(&self, _: LogicalTime, _: Vec<Payload>) {}

    fn new_epoch(&self, _: Arc<BatchReader>, _: Sender<WrapperCommand>) {}

    async fn update_payload(&self, _: &Block) {}

    async fn get_data(&self, block: &Block) -> Result<Vec<SignedTransaction>, Error> {
        if block.payload().is_none() {
            Ok(Vec::new())
        } else {
            let payload = block.payload().unwrap().clone();
            match payload {
                Payload::Empty => Ok(Vec::new()),
                Payload::DirectMempool(txns) => Ok(txns),
                Payload::InQuorumStore(_) => {
                    unreachable!("Quorum store should not be used.")
                }
            }
        }
    }
}
