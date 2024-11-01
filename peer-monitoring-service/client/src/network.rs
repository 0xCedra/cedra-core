// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    logging::{LogEntry, LogEvent, LogSchema},
    metrics, Error,
};
use aptos_config::network_id::PeerNetworkId;
use aptos_infallible::RwLock;
use aptos_logger::{trace, warn};
use aptos_network::{
    application::{
        interface::{NetworkClient, NetworkClientInterface},
        metadata::ConnectionState,
        storage::PeersAndMetadata,
    },
    peer::DisconnectReason,
};
use aptos_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest, response::PeerMonitoringServiceResponse,
    PeerMonitoringServiceMessage,
};
use std::{sync::Arc, time::Duration};

/// The interface for sending peer monitoring service requests
/// and querying peer information.
#[derive(Clone, Debug)]
pub struct PeerMonitoringServiceClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<PeerMonitoringServiceMessage>>
    PeerMonitoringServiceClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    /// Sends an RPC request to the specified peer with the given timeout
    pub async fn send_request(
        &self,
        recipient: PeerNetworkId,
        request: PeerMonitoringServiceRequest,
        timeout: Duration,
    ) -> Result<PeerMonitoringServiceResponse, Error> {
        let response = self
            .network_client
            .send_to_peer_rpc(
                PeerMonitoringServiceMessage::Request(request),
                timeout,
                recipient,
            )
            .await
            .map_err(|error| Error::NetworkError(error.to_string()))?;
        match response {
            PeerMonitoringServiceMessage::Response(Ok(response)) => Ok(response),
            PeerMonitoringServiceMessage::Response(Err(err)) => {
                Err(Error::PeerMonitoringServiceError(err))
            },
            PeerMonitoringServiceMessage::Request(request) => Err(Error::NetworkError(format!(
                "Got peer monitoring request instead of response! Request: {:?}",
                request
            ))),
        }
    }

    /// Returns the peers and metadata struct
    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.network_client.get_peers_and_metadata()
    }

    /// Disconnect from peer
    pub async fn disconnect_from_peer(
        &self,
        peer: PeerNetworkId,
        reason: DisconnectReason,
    ) -> Result<(), aptos_network::application::error::Error> {
        self.network_client
            .get_peers_and_metadata()
            .update_connection_state(peer, ConnectionState::Disconnecting)?;
        self.network_client.disconnect_from_peer(peer, reason).await
    }
}

/// Sends a request to a specific peer
pub async fn send_request_to_peer(
    peer_monitoring_client: Arc<
        RwLock<PeerMonitoringServiceClient<NetworkClient<PeerMonitoringServiceMessage>>>,
    >,
    peer_network_id: &PeerNetworkId,
    request_id: u64,
    request: PeerMonitoringServiceRequest,
    request_timeout_ms: u64,
) -> Result<PeerMonitoringServiceResponse, Error> {
    trace!(
        (LogSchema::new(LogEntry::SendRequest)
            .event(LogEvent::SendRequest)
            .request_type(request.get_label())
            .request_id(request_id)
            .peer(peer_network_id)
            .request(&request))
    );
    metrics::increment_request_counter(
        &metrics::SENT_REQUESTS,
        request.get_label(),
        peer_network_id,
    );

    let client = {
        let read_guard = peer_monitoring_client.read();
        read_guard.clone()
    };

    // Send the request and process the result
    let result = client
        .send_request(
            *peer_network_id,
            request.clone(),
            Duration::from_millis(request_timeout_ms),
        )
        .await;
    match result {
        Ok(response) => {
            trace!(
                (LogSchema::new(LogEntry::SendRequest)
                    .event(LogEvent::ResponseSuccess)
                    .request_type(request.get_label())
                    .request_id(request_id)
                    .peer(peer_network_id))
            );
            metrics::increment_request_counter(
                &metrics::SUCCESS_RESPONSES,
                request.clone().get_label(),
                peer_network_id,
            );
            Ok(response)
        },
        Err(error) => {
            warn!(
                (LogSchema::new(LogEntry::SendRequest)
                    .event(LogEvent::ResponseError)
                    .request_type(request.get_label())
                    .request_id(request_id)
                    .peer(peer_network_id)
                    .error(&error))
            );
            metrics::increment_request_counter(
                &metrics::ERROR_RESPONSES,
                error.get_label(),
                peer_network_id,
            );
            Err(error)
        },
    }
}
