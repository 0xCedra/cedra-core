// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::ConsensusMsg,
    quorum_store::{
        batch_store::BatchReader,
        proof_coordinator::{ProofCoordinator, ProofCoordinatorCommand},
        tests::utils::create_vec_signed_transactions,
        types::Batch,
    },
    test_utils::mock_quorum_store_sender::MockQuorumStoreSender,
};
use aptos_consensus_types::proof_of_store::{BatchId, LogicalTime, ProofOfStore, SignedBatchInfo};
use aptos_crypto::HashValue;
use aptos_executor_types::Error;
use aptos_types::{
    transaction::SignedTransaction, validator_verifier::random_validator_verifier, PeerId,
};
use std::sync::Arc;
use tokio::sync::{mpsc::channel, oneshot::Receiver};

pub struct MockBatchReader {
    peer: PeerId,
}

impl BatchReader for MockBatchReader {
    fn exists(&self, _digest: &HashValue) -> Option<PeerId> {
        Some(self.peer)
    }

    fn get_batch(&self, _proof: ProofOfStore) -> Receiver<Result<Vec<SignedTransaction>, Error>> {
        unimplemented!();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_proof_coordinator_basic() {
    aptos_logger::Logger::init_for_testing();
    let (signers, verifier) = random_validator_verifier(4, None, true);
    let (tx, _rx) = channel(100);
    let proof_coordinator = ProofCoordinator::new(
        100,
        signers[0].author(),
        Arc::new(MockBatchReader {
            peer: signers[0].author(),
        }),
        tx,
    );
    let (proof_coordinator_tx, proof_coordinator_rx) = channel(100);
    let (tx, mut rx) = channel(100);
    let network_sender = MockQuorumStoreSender::new(tx);
    tokio::spawn(proof_coordinator.start(proof_coordinator_rx, network_sender, verifier.clone()));

    let batch_author = signers[0].author();
    let batch_id = BatchId::new_for_test(1);
    let payload = create_vec_signed_transactions(100);
    let batch = Batch::new(batch_id, payload, LogicalTime::new(1, 20), batch_author);
    let digest = batch.digest();

    for signer in &signers {
        let signed_batch_info = SignedBatchInfo::new(batch.info().clone(), signer).unwrap();
        assert!(proof_coordinator_tx
            .send(ProofCoordinatorCommand::AppendSignature(signed_batch_info))
            .await
            .is_ok());
    }

    let proof = match rx.recv().await.expect("channel dropped") {
        (ConsensusMsg::ProofOfStoreMsg(proof), _) => *proof,
        msg => panic!("Expected LocalProof but received: {:?}", msg),
    };
    // check normal path
    assert_eq!(proof.digest().clone(), digest);
    assert!(proof.verify(&verifier).is_ok());
}
