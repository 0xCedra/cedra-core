// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::proof_of_store::ProofOfStore;
use aptos_crypto::HashValue;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::fmt::Write;

/// The round of a block is a consensus-internal counter, which starts with 0 and increases
/// monotonically. It is used for the protocol safety and liveness (please see the detailed
/// protocol description).
pub type Round = u64;
/// Author refers to the author's account address
pub type Author = AccountAddress;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionSummary {
    pub sender: AccountAddress,
    pub sequence_number: u64,
}

impl fmt::Display for TransactionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.sender, self.sequence_number,)
    }
}

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    Empty,
    DirectMempool(Vec<SignedTransaction>),
}

impl Payload {
    pub fn empty() -> Self {
        Payload::Empty
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Payload::DirectMempool(txns) => txns.is_empty(),
            Payload::InQuorumStore(proofs) => proofs.is_empty(),
            Payload::Empty => true,
        }
    }

    pub fn is_direct(&self) -> bool {
        match self {
            Payload::DirectMempool(_) => true,
            Payload::InQuorumStore(_) => false,
            Payload::Empty => false,
        }
    }

    /// This is computationally expensive on the first call
    pub fn size(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns
                .par_iter()
                .with_min_len(100)
                .map(|txn| txn.raw_txn_bytes_len())
                .sum(),
            Payload::InQuorumStore(_) => 0, // quorum store TODO
            Payload::Empty => 0,
        }
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            Payload::Empty => Ok(()),
            Payload::DirectMempool(_) => Ok(()),
            Payload::InQuorumStore(proofs) => {
                for proof in proofs.iter() {
                    proof.verify(validator)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(txns) => {
                write!(f, "InMemory txns: {}", txns.len())
            }
            Payload::InQuorumStore(proofs) => {
                write!(f, "InMemory poavs: {}", proofs.len())
            }
            Payload::Empty => write!(f, "Empty payload"),
        }
    }
}

/// The payload to filter.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PayloadFilter {
    DirectMempool(Vec<TransactionSummary>),
    InQuorumStore(HashSet<HashValue>),
    //
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
                if let Payload::InQuorumStore(proofs) = payload {
                    for proof in proofs {
                        exclude_proofs.insert(*proof.digest());
                    }
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
            }
            PayloadFilter::InQuorumStore(excluded_proofs) => {
                let mut txns_str = "".to_string();
                for proof in excluded_proofs.iter() {
                    write!(txns_str, "{} ", proof)?;
                }
                write!(f, "{}", txns_str)
            }
            PayloadFilter::Empty => {
                write!(f, "Empty filter")
            }
        }
    }
}
