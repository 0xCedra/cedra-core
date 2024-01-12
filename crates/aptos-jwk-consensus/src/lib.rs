// Copyright © Aptos Foundation

use crate::{
    epoch_manager::EpochManager, network::NetworkTask, network_interface::JWKConsensusNetworkClient,
};
use aptos_config::config::IdentityBlob;
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_types::account_address::AccountAddress;
use aptos_validator_transaction_pool as vtxn_pool;
use std::sync::Arc;
use tokio::runtime::Runtime;
use types::JWKConsensusMsg;

#[allow(clippy::let_and_return)]
pub fn start_jwk_consensus_runtime(
    my_addr: AccountAddress,
    identity_blob: Arc<IdentityBlob>,
    network_client: NetworkClient<JWKConsensusMsg>,
    network_service_events: NetworkServiceEvents<JWKConsensusMsg>,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    jwk_updated_events: EventNotificationListener,
    vtxn_pool_writer: vtxn_pool::SingleTopicWriteClient,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("jwk".into(), Some(4));
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    let jwk_consensus_network_client = JWKConsensusNetworkClient::new(network_client);
    let epoch_manager = EpochManager::new(
        my_addr,
        identity_blob,
        reconfig_events,
        jwk_updated_events,
        self_sender,
        jwk_consensus_network_client,
        vtxn_pool_writer,
    );
    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);
    runtime.spawn(network_task.start());
    runtime.spawn(epoch_manager.start(network_receiver));
    runtime
}

pub mod certified_update_producer;
pub mod counters;
pub mod epoch_manager;
pub mod jwk_manager;
pub mod jwk_observer;
pub mod network;
pub mod network_interface;
pub mod observation_aggregation;
pub mod signing_key_provider;
pub mod types;
