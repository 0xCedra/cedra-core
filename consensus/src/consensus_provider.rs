// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    epoch_manager::EpochManager,
    network::NetworkTask,
    network_interface::{ConsensusNetworkEvents, ConsensusNetworkSender},
    persistent_liveness_storage::StorageWriteProxy,
    state_computer::ExecutionProxy,
    txn_notifier::MempoolNotifier,
    util::time_service::ClockTimeService,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::config::NodeConfig;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_event_notifications::ReconfigNotificationListener;
use aptos_executor::block_executor::BlockExecutor;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_network::application::storage::PeerMetadataStorage;
use aptos_storage_interface::DbReaderWriter;
use aptos_vm::AptosVM;
use futures::channel::mpsc;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::runtime::{self, Runtime};

/// Helper function to start consensus based on configuration and return the runtime
pub fn start_consensus(
    node_config: &NodeConfig,
    mut network_sender: ConsensusNetworkSender,
    network_events: ConsensusNetworkEvents,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    consensus_to_mempool_sender: mpsc::Sender<QuorumStoreRequest>,
    aptos_db: DbReaderWriter,
    reconfig_events: ReconfigNotificationListener,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) -> Runtime {
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("consensus-{}", id)
        })
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    let storage = Arc::new(StorageWriteProxy::new(node_config, aptos_db.reader.clone()));
    let txn_notifier = Arc::new(MempoolNotifier::new(
        consensus_to_mempool_sender.clone(),
        node_config.consensus.mempool_executed_txn_timeout_ms,
    ));

    let state_computer = Arc::new(ExecutionProxy::new(
        Arc::new(BlockExecutor::<AptosVM>::new(aptos_db)),
        txn_notifier,
        state_sync_notifier,
        runtime.handle(),
    ));
    let bounded_executor = BoundedExecutor::new(4, runtime.handle().clone());

    let time_service = Arc::new(ClockTimeService::new(runtime.handle().clone()));

    let (timeout_sender, timeout_receiver) =
        aptos_channels::new(1_024, &counters::PENDING_ROUND_TIMEOUTS);
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    network_sender.initialize(peer_metadata_storage);

    let epoch_mgr = EpochManager::new(
        node_config,
        time_service,
        self_sender,
        network_sender,
        timeout_sender,
        consensus_to_mempool_sender,
        state_computer,
        storage,
        reconfig_events,
        bounded_executor,
    );

    let (network_task, network_receiver) = NetworkTask::new(network_events, self_receiver);

    runtime.spawn(network_task.start());
    runtime.spawn(epoch_mgr.start(timeout_receiver, network_receiver));

    debug!("Consensus started.");
    runtime
}
