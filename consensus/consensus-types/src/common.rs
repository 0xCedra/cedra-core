// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::proof_of_store::{BatchInfo, ProofOfStore};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_executor_types::ExecutorResult;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, transaction::SignedTransaction,
    validator_verifier::ValidatorVerifier, vm_status::DiscardedVMStatus, PeerId,
};
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashSet, fmt, fmt::Write, sync::Arc};
use tokio::sync::oneshot;

/// The round of a block is a consensus-internal counter, which starts with 0 and increases
/// monotonically. It is used for the protocol safety and liveness (please see the detailed
/// protocol description).
pub type Round = u64;
/// Author refers to the author's account address
pub type Author = AccountAddress;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash, Ord, PartialOrd)]
pub struct TransactionSummary {
    pub sender: AccountAddress,
    pub sequence_number: u64,
}

impl TransactionSummary {
    pub fn new(sender: AccountAddress, sequence_number: u64) -> Self {
        Self {
            sender,
            sequence_number,
        }
    }
}

impl fmt::Display for TransactionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.sender, self.sequence_number,)
    }
}

#[derive(Clone)]
pub struct TransactionInProgress {
    pub gas_unit_price: u64,
    pub count: u64,
}

impl TransactionInProgress {
    pub fn new(gas_unit_price: u64) -> Self {
        Self {
            gas_unit_price,
            count: 0,
        }
    }

    pub fn gas_unit_price(&self) -> u64 {
        self.gas_unit_price
    }

    pub fn decrement(&mut self) -> u64 {
        self.count -= 1;
        self.count
    }

    pub fn increment(&mut self) -> u64 {
        self.count += 1;
        self.count
    }
}

#[derive(Clone)]
pub struct RejectedTransactionSummary {
    pub sender: AccountAddress,
    pub sequence_number: u64,
    pub hash: HashValue,
    pub reason: DiscardedVMStatus,
}

#[derive(Debug)]
pub enum DataStatus {
    Cached(Vec<SignedTransaction>),
    Requested(
        Vec<(
            HashValue,
            oneshot::Receiver<ExecutorResult<Vec<SignedTransaction>>>,
        )>,
    ),
}

impl DataStatus {
    pub fn extend(&mut self, other: DataStatus) {
        match (self, other) {
            (DataStatus::Requested(v1), DataStatus::Requested(v2)) => v1.extend(v2),
            (_, _) => unreachable!(),
        }
    }

    pub fn take(&mut self) -> DataStatus {
        std::mem::replace(self, DataStatus::Requested(vec![]))
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProofWithData {
    pub proofs: Vec<ProofOfStore>,
    #[serde(skip)]
    pub status: Arc<Mutex<Option<DataStatus>>>,
}

impl PartialEq for ProofWithData {
    fn eq(&self, other: &Self) -> bool {
        self.proofs == other.proofs && Arc::as_ptr(&self.status) == Arc::as_ptr(&other.status)
    }
}

impl Eq for ProofWithData {}

impl ProofWithData {
    pub fn new(proofs: Vec<ProofOfStore>) -> Self {
        Self {
            proofs,
            status: Arc::new(Mutex::new(None)),
        }
    }

    pub fn extend(&mut self, other: ProofWithData) {
        let other_data_status = other.status.lock().as_mut().unwrap().take();
        self.proofs.extend(other.proofs);
        let mut status = self.status.lock();
        if status.is_none() {
            *status = Some(other_data_status);
        } else {
            status.as_mut().unwrap().extend(other_data_status);
        }
    }

    pub fn len(&self) -> usize {
        self.proofs
            .iter()
            .map(|proof| proof.num_txns() as usize)
            .sum()
    }

    pub fn num_bytes(&self) -> usize {
        self.proofs
            .iter()
            .map(|proof| proof.num_bytes() as usize)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.proofs.is_empty()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProofWithDataWithTxnLimit {
    pub proof_with_data: ProofWithData,
    pub max_txns_to_execute: Option<usize>,
}

impl PartialEq for ProofWithDataWithTxnLimit {
    fn eq(&self, other: &Self) -> bool {
        self.proof_with_data == other.proof_with_data
            && self.max_txns_to_execute == other.max_txns_to_execute
    }
}

impl Eq for ProofWithDataWithTxnLimit {}

impl ProofWithDataWithTxnLimit {
    pub fn new(proof_with_data: ProofWithData, max_txns_to_execute: Option<usize>) -> Self {
        Self {
            proof_with_data,
            max_txns_to_execute,
        }
    }

    pub fn extend(&mut self, other: ProofWithDataWithTxnLimit) {
        self.proof_with_data.extend(other.proof_with_data);
        // InQuorumStoreWithLimit TODO: what is the right logic here ???
        if self.max_txns_to_execute.is_none() {
            self.max_txns_to_execute = other.max_txns_to_execute;
        }
    }
}

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    DirectMempool(Vec<SignedTransaction>),
    InQuorumStore(ProofWithData),
    InQuorumStoreWithLimit(ProofWithDataWithTxnLimit),
    QuorumStoreInlineHybrid(
        Vec<(BatchInfo, Vec<SignedTransaction>)>,
        ProofWithData,
        Option<usize>,
    ),
}

impl Payload {
    pub fn transform_to_quorum_store_v2(self, max_txns_to_execute: Option<usize>) -> Self {
        match self {
            Payload::InQuorumStore(proof_with_status) => Payload::InQuorumStoreWithLimit(
                ProofWithDataWithTxnLimit::new(proof_with_status, max_txns_to_execute),
            ),
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                Payload::QuorumStoreInlineHybrid(
                    inline_batches,
                    proof_with_data,
                    max_txns_to_execute,
                )
            },
            Payload::InQuorumStoreWithLimit(_) => {
                panic!("Payload is already in quorumStoreV2 format");
            },
            Payload::DirectMempool(_) => {
                panic!("Payload is in direct mempool format");
            },
        }
    }

    pub fn empty(quorum_store_enabled: bool, allow_batches_without_pos_in_proposal: bool) -> Self {
        if quorum_store_enabled {
            if allow_batches_without_pos_in_proposal {
                Payload::QuorumStoreInlineHybrid(Vec::new(), ProofWithData::new(Vec::new()), None)
            } else {
                Payload::InQuorumStore(ProofWithData::new(Vec::new()))
            }
        } else {
            Payload::DirectMempool(Vec::new())
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns.len(),
            Payload::InQuorumStore(proof_with_status) => proof_with_status.len(),
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                let num_txns = proof_with_status.proof_with_data.len();
                if proof_with_status.max_txns_to_execute.is_some() {
                    min(proof_with_status.max_txns_to_execute.unwrap(), num_txns)
                } else {
                    num_txns
                }
            },
            Payload::QuorumStoreInlineHybrid(
                inline_batches,
                proof_with_data,
                max_txns_to_execute,
            ) => {
                let num_txns = proof_with_data.len()
                    + inline_batches
                        .iter()
                        .map(|(_, txns)| txns.len())
                        .sum::<usize>();
                if max_txns_to_execute.is_some() {
                    min(max_txns_to_execute.unwrap(), num_txns)
                } else {
                    num_txns
                }
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Payload::DirectMempool(txns) => txns.is_empty(),
            Payload::InQuorumStore(proof_with_status) => proof_with_status.proofs.is_empty(),
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                proof_with_status.proof_with_data.proofs.is_empty()
                    || proof_with_status.max_txns_to_execute == Some(0)
            },
            Payload::QuorumStoreInlineHybrid(
                inline_batches,
                proof_with_data,
                max_txns_to_execute,
            ) => {
                *max_txns_to_execute == Some(0)
                    || (proof_with_data.proofs.is_empty() && inline_batches.is_empty())
            },
        }
    }

    pub fn extend(self, other: Payload) -> Self {
        match (self, other) {
            (Payload::DirectMempool(v1), Payload::DirectMempool(v2)) => {
                let mut v3 = v1;
                v3.extend(v2);
                Payload::DirectMempool(v3)
            },
            (Payload::InQuorumStore(p1), Payload::InQuorumStore(p2)) => {
                let mut p3 = p1;
                p3.extend(p2);
                Payload::InQuorumStore(p3)
            },
            (Payload::InQuorumStoreWithLimit(p1), Payload::InQuorumStoreWithLimit(p2)) => {
                let mut p3 = p1;
                p3.extend(p2);
                Payload::InQuorumStoreWithLimit(p3)
            },
            (
                Payload::QuorumStoreInlineHybrid(b1, p1, m1),
                Payload::QuorumStoreInlineHybrid(b2, p2, m2),
            ) => {
                let mut b3 = b1;
                b3.extend(b2);
                let mut p3 = p1;
                p3.extend(p2);
                // TODO: What's the right logic here?
                let m3 = if m1.is_none() {
                    m2
                } else if let Some(m2) = m2 {
                    Some(m1.unwrap() + m2)
                } else {
                    m1
                };
                Payload::QuorumStoreInlineHybrid(b3, p3, m3)
            },
            (Payload::QuorumStoreInlineHybrid(b1, p1, m1), Payload::InQuorumStore(p2)) => {
                // TODO: How to update m1?
                let mut p3 = p1;
                p3.extend(p2);
                Payload::QuorumStoreInlineHybrid(b1, p3, m1)
            },
            (Payload::QuorumStoreInlineHybrid(b1, p1, m1), Payload::InQuorumStoreWithLimit(p2)) => {
                // TODO: What's the right logic here?
                let m3 = if m1.is_none() {
                    p2.max_txns_to_execute
                } else if let Some(m2) = p2.max_txns_to_execute {
                    Some(m1.unwrap() + m2)
                } else {
                    m1
                };
                let mut p3 = p1;
                p3.extend(p2.proof_with_data);
                Payload::QuorumStoreInlineHybrid(b1, p3, m3)
            },
            (Payload::InQuorumStore(p1), Payload::QuorumStoreInlineHybrid(b2, p2, m2)) => {
                let mut p3 = p1;
                p3.extend(p2);
                Payload::QuorumStoreInlineHybrid(b2, p3, m2)
            },
            (Payload::InQuorumStoreWithLimit(p1), Payload::QuorumStoreInlineHybrid(b2, p2, m2)) => {
                // TODO: What's the right logic here?
                let m3 = if p1.max_txns_to_execute.is_none() {
                    m2
                } else if m2.is_some() {
                    Some(p1.max_txns_to_execute.unwrap() + m2.unwrap())
                } else {
                    p1.max_txns_to_execute
                };
                let mut p3 = p1.proof_with_data;
                p3.extend(p2);
                Payload::QuorumStoreInlineHybrid(b2, p3, m3)
            },
            (_, _) => unreachable!(),
        }
    }

    pub fn is_direct(&self) -> bool {
        matches!(self, Payload::DirectMempool(_))
    }

    /// This is computationally expensive on the first call
    pub fn size(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns
                .par_iter()
                .with_min_len(100)
                .map(|txn| txn.raw_txn_bytes_len())
                .sum(),
            Payload::InQuorumStore(proof_with_status) => proof_with_status.num_bytes(),
            // We dedeup, shuffle and finally truncate the txns in the payload to the length == 'max_txns_to_execute'.
            // Hence, it makes sense to pass the full size of the payload here.
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                proof_with_status.proof_with_data.num_bytes()
            },
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                proof_with_data.num_bytes()
                    + inline_batches
                        .iter()
                        .map(|(batch_info, _)| batch_info.num_bytes() as usize)
                        .sum::<usize>()
            },
        }
    }

    pub fn verify(
        &self,
        validator: &ValidatorVerifier,
        quorum_store_enabled: bool,
    ) -> anyhow::Result<()> {
        match (quorum_store_enabled, self) {
            (false, Payload::DirectMempool(_)) => Ok(()),
            (true, Payload::InQuorumStore(proof_with_status)) => {
                proof_with_status
                    .proofs
                    .par_iter()
                    .with_min_len(4)
                    .try_for_each(|proof| proof.verify(validator))?;
                Ok(())
            },
            (true, Payload::InQuorumStoreWithLimit(proof_with_status)) => {
                proof_with_status
                    .proof_with_data
                    .proofs
                    .par_iter()
                    .with_min_len(4)
                    .try_for_each(|proof| proof.verify(validator))?;
                Ok(())
            },
            (true, Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _)) => {
                for proof in proof_with_data.proofs.iter() {
                    proof.verify(validator)?;
                }
                for (batch, payload) in inline_batches.iter() {
                    // TODO: Can cloning be avoided here?
                    if BatchPayload::new(batch.author(), payload.clone()).hash() != *batch.digest()
                    {
                        return Err(anyhow::anyhow!(
                            "Hash of the received inline batch doesn't match the digest value",
                        ));
                    }
                }
                Ok(())
            },
            (_, _) => Err(anyhow::anyhow!(
                "Wrong payload type. Expected Payload::InQuorumStore {} got {} ",
                quorum_store_enabled,
                self
            )),
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(txns) => {
                write!(f, "InMemory txns: {}", txns.len())
            },
            Payload::InQuorumStore(proof_with_status) => {
                write!(f, "InMemory proofs: {}", proof_with_status.proofs.len())
            },
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                write!(
                    f,
                    "InMemory proofs: {}",
                    proof_with_status.proof_with_data.proofs.len()
                )
            },
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                write!(
                    f,
                    "Inline txns: {}, InMemory proofs: {}",
                    inline_batches
                        .iter()
                        .map(|(_, txns)| txns.len())
                        .sum::<usize>(),
                    proof_with_data.proofs.len()
                )
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher)]
pub struct BatchPayload {
    author: PeerId,
    txns: Vec<SignedTransaction>,
    #[serde(skip)]
    num_bytes: OnceCell<usize>,
}

impl CryptoHash for BatchPayload {
    type Hasher = BatchPayloadHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::new();
        let bytes = bcs::to_bytes(&self).expect("Unable to serialize batch payload");
        self.num_bytes.get_or_init(|| bytes.len());
        state.update(&bytes);
        state.finish()
    }
}

impl BatchPayload {
    pub fn new(author: PeerId, txns: Vec<SignedTransaction>) -> Self {
        Self {
            author,
            txns,
            num_bytes: OnceCell::new(),
        }
    }

    pub fn into_transactions(self) -> Vec<SignedTransaction> {
        self.txns
    }

    pub fn txns(&self) -> &Vec<SignedTransaction> {
        &self.txns
    }

    pub fn num_txns(&self) -> usize {
        self.txns.len()
    }

    pub fn num_bytes(&self) -> usize {
        *self
            .num_bytes
            .get_or_init(|| bcs::serialized_size(&self).expect("unable to serialize batch payload"))
    }

    pub fn author(&self) -> PeerId {
        self.author
    }
}

/// The payload to filter.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PayloadFilter {
    DirectMempool(Vec<TransactionSummary>),
    InQuorumStore(HashSet<BatchInfo>),
    Empty,
}

impl From<&Vec<&Payload>> for PayloadFilter {
    fn from(exclude_payloads: &Vec<&Payload>) -> Self {
        if exclude_payloads.is_empty() {
            return PayloadFilter::Empty;
        }
        let direct_mode = exclude_payloads.iter().any(|payload| payload.is_direct());

        if direct_mode {
            let mut exclude_txns = Vec::new();
            for payload in exclude_payloads {
                if let Payload::DirectMempool(txns) = payload {
                    for txn in txns {
                        exclude_txns.push(TransactionSummary {
                            sender: txn.sender(),
                            sequence_number: txn.sequence_number(),
                        });
                    }
                }
            }
            PayloadFilter::DirectMempool(exclude_txns)
        } else {
            let mut exclude_proofs = HashSet::new();
            for payload in exclude_payloads {
                match payload {
                    Payload::InQuorumStore(proof_with_status) => {
                        for proof in &proof_with_status.proofs {
                            exclude_proofs.insert(proof.info().clone());
                        }
                    },
                    Payload::InQuorumStoreWithLimit(proof_with_status) => {
                        for proof in &proof_with_status.proof_with_data.proofs {
                            exclude_proofs.insert(proof.info().clone());
                        }
                    },
                    Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                        for proof in &proof_with_data.proofs {
                            exclude_proofs.insert(proof.info().clone());
                        }
                        for (batch_info, _) in inline_batches {
                            exclude_proofs.insert(batch_info.clone());
                        }
                    },
                    Payload::DirectMempool(_) => {
                        error!("DirectMempool payload in InQuorumStore filter");
                    },
                }
            }
            PayloadFilter::InQuorumStore(exclude_proofs)
        }
    }
}

impl fmt::Display for PayloadFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadFilter::DirectMempool(excluded_txns) => {
                let mut txns_str = "".to_string();
                for tx in excluded_txns.iter() {
                    write!(txns_str, "{} ", tx)?;
                }
                write!(f, "{}", txns_str)
            },
            PayloadFilter::InQuorumStore(excluded_proofs) => {
                let mut proofs_str = "".to_string();
                for proof in excluded_proofs.iter() {
                    write!(proofs_str, "{} ", proof.digest())?;
                }
                write!(f, "{}", proofs_str)
            },
            PayloadFilter::Empty => {
                write!(f, "Empty filter")
            },
        }
    }
}
