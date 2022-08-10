// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{quorum_store::QuorumStoreError, types::BatchId, utils::DigestTimeouts};
use aptos_crypto::{bls12381, HashValue};
use aptos_logger::{debug, info};
use aptos_types::multi_signature::PartialSignatures;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::PeerId;
use consensus_types::proof_of_store::{
    ProofOfStore, SignedDigest, SignedDigestError, SignedDigestInfo,
};
use futures::channel::oneshot;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::{sync::oneshot as TokioOneshot, time};

#[derive(Debug)]
pub(crate) enum ProofBuilderCommand {
    InitProof(SignedDigestInfo, BatchId, ProofReturnChannel),
    AppendSignature(SignedDigest),
    Shutdown(TokioOneshot::Sender<()>),
}

pub(crate) type ProofReturnChannel =
    oneshot::Sender<Result<(ProofOfStore, BatchId), QuorumStoreError>>;

struct IncrementalProofState {
    info: SignedDigestInfo,
    aggregated_signature: HashMap<PeerId, bls12381::Signature>,
    batch_id: BatchId,
    ret_tx: ProofReturnChannel,
}

impl IncrementalProofState {
    fn new(info: SignedDigestInfo, batch_id: BatchId, ret_tx: ProofReturnChannel) -> Self {
        Self {
            info,
            aggregated_signature: HashMap::new(),
            batch_id,
            ret_tx,
        }
    }

    fn add_signature(
        &mut self,
        signer_id: PeerId,
        signature: bls12381::Signature,
    ) -> Result<(), SignedDigestError> {
        if self.aggregated_signature.contains_key(&signer_id) {
            return Err(SignedDigestError::DuplicatedSignature);
        }

        self.aggregated_signature.insert(signer_id, signature);
        Ok(())
    }

    fn ready(&self, validator_verifier: &ValidatorVerifier, my_peer_id: PeerId) -> bool {
        self.aggregated_signature.contains_key(&my_peer_id)
            && validator_verifier
                .check_voting_power(self.aggregated_signature.keys())
                .is_ok()
    }

    fn take(
        self,
        validator_verifier: &ValidatorVerifier,
    ) -> (ProofOfStore, BatchId, ProofReturnChannel) {
        let proof = match validator_verifier
            .aggregate_multi_signature(&PartialSignatures::new(self.aggregated_signature))
        {
            Ok((sig, _)) => ProofOfStore::new(self.info, sig),
            Err(e) => unreachable!("Cannot aggregate signatures on digest err = {:?}", e),
        };
        (proof, self.batch_id, self.ret_tx)
    }

    fn send_timeout(self) {
        self.ret_tx
            .send(Err(QuorumStoreError::Timeout(self.batch_id)))
            .expect("Unable to send the timeout a proof of store");
    }
}

pub(crate) struct ProofBuilder {
    peer_id: PeerId,
    proof_timeout_ms: usize,
    digest_to_proof: HashMap<HashValue, IncrementalProofState>,
    timeouts: DigestTimeouts,
}

//PoQS builder object - gather signed digest to form PoQS
impl ProofBuilder {
    pub fn new(proof_timeout_ms: usize, peer_id: PeerId) -> Self {
        Self {
            peer_id,
            proof_timeout_ms,
            digest_to_proof: HashMap::new(),
            timeouts: DigestTimeouts::new(),
        }
    }

    fn init_proof(
        &mut self,
        info: SignedDigestInfo,
        batch_id: BatchId,
        tx: ProofReturnChannel,
    ) -> Result<(), SignedDigestError> {
        self.timeouts.add_digest(info.digest, self.proof_timeout_ms);
        self.digest_to_proof
            .insert(info.digest, IncrementalProofState::new(info, batch_id, tx));
        Ok(())
    }

    fn add_signature(
        &mut self,
        signed_digest: SignedDigest,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<(), SignedDigestError> {
        if !self
            .digest_to_proof
            .contains_key(&signed_digest.info.digest)
        {
            return Err(SignedDigestError::WrongDigest);
        }
        let mut ret = Ok(());
        let mut ready = false;
        let digest = signed_digest.info.digest.clone();
        let my_id = self.peer_id;
        self.digest_to_proof
            .entry(signed_digest.info.digest)
            .and_modify(|state| {
                ret = state.add_signature(signed_digest.peer_id, signed_digest.signature);
                if ret.is_ok() {
                    ready = state.ready(validator_verifier, my_id);
                }
            });
        if ready {
            let (proof, batch_id, tx) = self
                .digest_to_proof
                .remove(&digest)
                .unwrap()
                .take(validator_verifier);
            tx.send(Ok((proof, batch_id)))
                .expect("Unable to send the proof of store");
        }
        ret
    }

    fn expire(&mut self) {
        for digest in self.timeouts.expire() {
            if let Some(state) = self.digest_to_proof.remove(&digest) {
                state.send_timeout();
            }
        }
    }

    pub async fn start(
        mut self,
        mut rx: Receiver<ProofBuilderCommand>,
        validator_verifier: ValidatorVerifier,
    ) {
        let mut interval = time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
             Some(command) = rx.recv() => {
                match command {
                        ProofBuilderCommand::Shutdown(ack_tx) => {
                    ack_tx
                        .send(())
                        .expect("Failed to send shutdown ack to QuorumStore");
                    break;
                }
                    ProofBuilderCommand::InitProof(info, batch_id, tx) => {
                        self.init_proof(info, batch_id, tx)
                            .expect("Error initializing proof of store");
                    }
                    ProofBuilderCommand::AppendSignature(signed_digest) => {
                            let peer_id = signed_digest.peer_id;
                        if let Err(e) = self.add_signature(signed_digest, &validator_verifier) {
                            // Can happen if we already garbage collected
                            if peer_id == self.peer_id {
                                info!("QS: could not add signature from self, err = {:?}", e);
                                }
                        } else {
                            debug!("QS: added signature to proof");
                        }
                    }
                }

                }
                _ = interval.tick() => {
                    self.expire();
                }
            }
        }
    }
}
