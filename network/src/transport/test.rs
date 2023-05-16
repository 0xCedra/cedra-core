// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolId, ProtocolIdSet},
    testutils,
    transport::*,
};
use aptos_config::config::{Peer, PeerRole, PeerSet, HANDSHAKE_VERSION};
use aptos_crypto::{test_utils::TEST_SEED, traits::Uniform, x25519};
use aptos_infallible::RwLock;
use aptos_netcore::{
    framing::{read_u16frame, write_u16frame},
    transport::{memory, ConnectionOrigin, Transport},
};
use aptos_time_service::MockTimeService;
use aptos_types::{
    chain_id::ChainId,
    network_address::{NetworkAddress, Protocol::*},
    PeerId,
};
use bytes::{Bytes, BytesMut};
use futures::{future, io::AsyncWriteExt, stream::StreamExt};
use rand::{rngs::StdRng, SeedableRng};
use std::{io, iter::FromIterator, sync::Arc};
use tokio::runtime::Runtime;

/// Helper to add the trusted peer to the set
fn add_trusted_peer(
    trusted_peers: Arc<RwLock<PeerSet>>,
    peer_id: PeerId,
    private_key: &x25519::PrivateKey,
    role: PeerRole,
) {
    let pubkey_set = [private_key.public_key()].iter().copied().collect();
    let peer = Peer::new(Vec::new(), pubkey_set, role);
    trusted_peers.write().insert(peer_id, peer);
}

enum Auth {
    Mutual,
    MaybeMutual,
    ServerOnly,
}

fn setup<TTransport>(
    base_transport: TTransport,
    auth: Auth,
) -> (
    Runtime,
    MockTimeService,
    (PeerId, AptosNetTransport<TTransport>),
    (PeerId, AptosNetTransport<TTransport>),
    Arc<RwLock<PeerSet>>,
    ProtocolIdSet,
)
where
    TTransport: Transport<Error = io::Error> + Clone,
    TTransport::Output: TSocket,
    TTransport::Outbound: Send + 'static,
    TTransport::Inbound: Send + 'static,
    TTransport::Listener: Send + 'static,
{
    let rt = Runtime::new().unwrap();
    let time_service = TimeService::mock();

    let mut rng = StdRng::from_seed(TEST_SEED);
    let listener_key = x25519::PrivateKey::generate(&mut rng);
    let dialer_key = x25519::PrivateKey::generate(&mut rng);

    let (
        listener_network_context,
        dialer_network_context,
        listener_auth_mode,
        dialer_auth_mode,
        trusted_peers,
    ) = match auth {
        Auth::Mutual => {
            // Create the dialer and listener network contexts
            let (dialer_network_context, listener_network_context, peers_and_metadata) =
                testutils::create_client_server_network_context(None, None, None);

            // Add the trusted peers
            let network_id = listener_network_context.network_id();
            let trusted_peers = peers_and_metadata.get_trusted_peers(&network_id).unwrap();
            add_trusted_peer(
                trusted_peers.clone(),
                dialer_network_context.peer_id(),
                &dialer_key,
                PeerRole::Validator,
            );
            add_trusted_peer(
                trusted_peers.clone(),
                listener_network_context.peer_id(),
                &listener_key,
                PeerRole::Validator,
            );

            (
                listener_network_context,
                dialer_network_context,
                HandshakeAuthMode::mutual(peers_and_metadata.clone()),
                HandshakeAuthMode::mutual(peers_and_metadata),
                trusted_peers,
            )
        },
        Auth::MaybeMutual => {
            // Create the dialer and listener network contexts
            let (dialer_network_context, listener_network_context, peers_and_metadata) =
                testutils::create_client_server_network_context(
                    Some(dialer_key.public_key()),
                    Some(listener_key.public_key()),
                    None,
                );

            // Add the trusted peers
            let network_id = listener_network_context.network_id();
            let trusted_peers = peers_and_metadata.get_trusted_peers(&network_id).unwrap();
            add_trusted_peer(
                trusted_peers.clone(),
                dialer_network_context.peer_id(),
                &dialer_key,
                PeerRole::Validator,
            );
            add_trusted_peer(
                trusted_peers.clone(),
                listener_network_context.peer_id(),
                &listener_key,
                PeerRole::Validator,
            );

            (
                listener_network_context,
                dialer_network_context,
                HandshakeAuthMode::maybe_mutual(peers_and_metadata.clone()),
                HandshakeAuthMode::maybe_mutual(peers_and_metadata),
                trusted_peers,
            )
        },
        Auth::ServerOnly => {
            // Create the dialer and listener network contexts
            let (dialer_network_context, listener_network_context, peers_and_metadata) =
                testutils::create_client_server_network_context(
                    Some(dialer_key.public_key()),
                    Some(listener_key.public_key()),
                    None,
                );

            // Get the trusted peers
            let network_id = listener_network_context.network_id();
            let trusted_peers = peers_and_metadata.get_trusted_peers(&network_id).unwrap();

            (
                listener_network_context,
                dialer_network_context,
                HandshakeAuthMode::server_only(&[network_id]),
                HandshakeAuthMode::server_only(&[network_id]),
                trusted_peers,
            )
        },
    };

    let supported_protocols =
        ProtocolIdSet::from_iter([ProtocolId::ConsensusRpcBcs, ProtocolId::DiscoveryDirectSend]);
    let chain_id = ChainId::default();
    let listener_transport = AptosNetTransport::new(
        base_transport.clone(),
        listener_network_context,
        time_service.clone(),
        listener_key,
        listener_auth_mode,
        HANDSHAKE_VERSION,
        chain_id,
        supported_protocols.clone(),
        false, /* Disable proxy protocol */
    );

    let dialer_transport = AptosNetTransport::new(
        base_transport,
        dialer_network_context,
        time_service.clone(),
        dialer_key,
        dialer_auth_mode,
        HANDSHAKE_VERSION,
        chain_id,
        supported_protocols.clone(),
        false, /* Disable proxy protocol */
    );

    (
        rt,
        time_service.into_mock(),
        (listener_network_context.peer_id(), listener_transport),
        (dialer_network_context.peer_id(), dialer_transport),
        trusted_peers,
        supported_protocols,
    )
}

async fn write_read_msg(socket: &mut impl TSocket, msg: &[u8]) -> Bytes {
    write_u16frame(socket, msg).await.unwrap();
    socket.flush().await.unwrap();

    let mut buf = BytesMut::new();
    read_u16frame(socket, &mut buf).await.unwrap();
    buf.freeze()
}

/// Check that the network address matches the format
/// `"/memory/<port>/noise-ik/<pubkey>/handshake/<version>"`
fn expect_memory_noise_addr(addr: &NetworkAddress) {
    assert!(
        matches!(addr.as_slice(), [Memory(_), NoiseIK(_), Handshake(_)]),
        "addr: '{}'",
        addr
    );
}

/// Check that the network address matches the format
/// `"/ip4/<ipaddr>/tcp/<port>/noise-ik/<pubkey>/handshake/<version>"`
fn expect_ip4_tcp_noise_addr(addr: &NetworkAddress) {
    assert!(
        matches!(addr.as_slice(), [Ip4(_), Tcp(_), NoiseIK(_), Handshake(_)]),
        "addr: '{}'",
        addr
    );
}

fn test_transport_success<TTransport>(
    base_transport: TTransport,
    auth: Auth,
    listen_addr: &str,
    expect_formatted_addr: fn(&NetworkAddress),
) where
    TTransport: Transport<Error = io::Error> + Clone,
    TTransport::Output: TSocket,
    TTransport::Outbound: Send + 'static,
    TTransport::Inbound: Send + 'static,
    TTransport::Listener: Send + 'static,
{
    let (
        rt,
        _mock_time,
        (listener_peer_id, listener_transport),
        (dialer_peer_id, dialer_transport),
        _peers_and_metadata,
        supported_protocols,
    ) = setup(base_transport, auth);

    let _guard = rt.enter();
    let (mut inbounds, listener_addr) = listener_transport
        .listen_on(listen_addr.parse().unwrap())
        .unwrap();
    expect_formatted_addr(&listener_addr);
    let supported_protocols_clone = supported_protocols.clone();

    // we accept the dialer's inbound connection, check the connection metadata,
    // and verify that the upgraded socket actually works (sends and receives
    // bytes).
    let listener_task = async move {
        // accept one inbound connection from dialer
        let (inbound, _dialer_addr) = inbounds.next().await.unwrap().unwrap();
        let mut conn = inbound.await.unwrap();

        // check connection metadata
        assert_eq!(conn.metadata.remote_peer_id, dialer_peer_id);
        expect_formatted_addr(&conn.metadata.addr);
        assert_eq!(conn.metadata.origin, ConnectionOrigin::Inbound);
        assert_eq!(
            conn.metadata.messaging_protocol,
            MessagingProtocolVersion::V1
        );
        assert_eq!(
            conn.metadata.application_protocols,
            supported_protocols_clone,
        );

        // test the socket works
        let msg = write_read_msg(&mut conn.socket, b"foobar").await;
        assert_eq!(&msg, b"barbaz".as_ref());
        conn.socket.close().await.unwrap();
    };

    // dial the listener, check the connection metadata, and verify that the
    // upgraded socket actually works (sends and receives bytes).
    let dialer_task = async move {
        // dial listener
        let mut conn = dialer_transport
            .dial(listener_peer_id, listener_addr.clone())
            .unwrap()
            .await
            .unwrap();

        // check connection metadata
        assert_eq!(conn.metadata.remote_peer_id, listener_peer_id);
        assert_eq!(conn.metadata.addr, listener_addr);
        assert_eq!(conn.metadata.origin, ConnectionOrigin::Outbound);
        assert_eq!(
            conn.metadata.messaging_protocol,
            MessagingProtocolVersion::V1
        );
        assert_eq!(conn.metadata.application_protocols, supported_protocols);

        // test the socket works
        let msg = write_read_msg(&mut conn.socket, b"barbaz").await;
        assert_eq!(&msg, b"foobar".as_ref());
        conn.socket.close().await.unwrap();
    };

    rt.block_on(future::join(listener_task, dialer_task));
}

fn test_transport_rejects_unauthed_dialer<TTransport>(
    base_transport: TTransport,
    listen_addr: &str,
    expect_formatted_addr: fn(&NetworkAddress),
) where
    TTransport: Transport<Error = io::Error> + Clone,
    TTransport::Output: TSocket,
    TTransport::Outbound: Send + 'static,
    TTransport::Inbound: Send + 'static,
    TTransport::Listener: Send + 'static,
{
    let (
        rt,
        _mock_time,
        (listener_peer_id, listener_transport),
        (dialer_peer_id, dialer_transport),
        trusted_peers,
        _supported_protocols,
    ) = setup(base_transport, Auth::Mutual);

    // remove dialer from trusted_peers set
    trusted_peers.write().remove(&dialer_peer_id).unwrap();

    let _guard = rt.enter();
    let (mut inbounds, listener_addr) = listener_transport
        .listen_on(listen_addr.parse().unwrap())
        .unwrap();
    expect_formatted_addr(&listener_addr);

    // we try to accept one inbound connection from the dialer. however, the
    // connection upgrade should fail because the dialer is not authenticated
    // (not in the trusted peers set).
    let listener_task = async move {
        let (inbound, _dialer_addr) = inbounds.next().await.unwrap().unwrap();
        inbound
            .await
            .expect_err("should fail because the dialer is not a trusted peer");
    };

    // we attempt to dial the listener. however, the connection upgrade should
    // fail because we are not authenticated.
    let dialer_task = async move {
        // dial listener
        let fut_upgrade = dialer_transport
            .dial(listener_peer_id, listener_addr.clone())
            .unwrap();
        fut_upgrade
            .await
            .expect_err("should fail because listener rejects our unauthed connection");
    };

    rt.block_on(future::join(listener_task, dialer_task));
}

fn test_transport_maybe_mutual<TTransport>(
    base_transport: TTransport,
    listen_addr: &str,
    expect_formatted_addr: fn(&NetworkAddress),
) where
    TTransport: Transport<Error = io::Error> + Clone,
    TTransport::Output: TSocket,
    TTransport::Outbound: Send + 'static,
    TTransport::Inbound: Send + 'static,
    TTransport::Listener: Send + 'static,
{
    let (
        rt,
        _mock_time,
        (listener_peer_id, listener_transport),
        (dialer_peer_id, dialer_transport),
        trusted_peers,
        supported_protocols,
    ) = setup(base_transport, Auth::MaybeMutual);

    let _guard = rt.enter();
    let (mut inbounds, listener_addr) = listener_transport
        .listen_on(listen_addr.parse().unwrap())
        .unwrap();
    expect_formatted_addr(&listener_addr);
    let supported_protocols_clone = supported_protocols.clone();

    // we accept the dialer's inbound connection, check the connection metadata,
    // and verify that the upgraded socket actually works (sends and receives
    // bytes).
    let listener_task = async move {
        // accept one inbound connection from dialer
        let (inbound, _dialer_addr) = inbounds.next().await.unwrap().unwrap();
        let mut conn = inbound.await.unwrap();

        // check connection metadata
        assert_eq!(conn.metadata.remote_peer_id, dialer_peer_id);
        expect_formatted_addr(&conn.metadata.addr);
        assert_eq!(conn.metadata.origin, ConnectionOrigin::Inbound);
        assert_eq!(
            conn.metadata.messaging_protocol,
            MessagingProtocolVersion::V1
        );
        assert_eq!(
            conn.metadata.application_protocols,
            supported_protocols_clone,
        );
        assert_eq!(
            conn.metadata.role,
            trusted_peers
                .read()
                .get(&conn.metadata.remote_peer_id)
                .unwrap()
                .role
        );

        // test the socket works
        let msg = write_read_msg(&mut conn.socket, b"foobar").await;
        assert_eq!(&msg, b"barbaz".as_ref());
        conn.socket.close().await.unwrap();

        // Clear the trusted peers and see that we can still connect to the remote but with it
        // being untrusted
        trusted_peers.write().clear();

        // accept one inbound connection from dialer
        let (inbound, _dialer_addr) = inbounds.next().await.unwrap().unwrap();
        let mut conn = inbound.await.unwrap();

        // check connection metadata
        assert_eq!(conn.metadata.remote_peer_id, dialer_peer_id);
        expect_formatted_addr(&conn.metadata.addr);
        assert_eq!(conn.metadata.origin, ConnectionOrigin::Inbound);
        assert_eq!(
            conn.metadata.messaging_protocol,
            MessagingProtocolVersion::V1
        );
        assert_eq!(
            conn.metadata.application_protocols,
            supported_protocols_clone,
        );
        assert_eq!(conn.metadata.role, PeerRole::Unknown);

        // test the socket works
        let msg = write_read_msg(&mut conn.socket, b"foobar").await;
        assert_eq!(&msg, b"barbaz".as_ref());
        conn.socket.close().await.unwrap();
    };

    // dial the listener, check the connection metadata, and verify that the
    // upgraded socket actually works (sends and receives bytes).
    let dialer_task = async move {
        // dial listener
        let mut conn = dialer_transport
            .dial(listener_peer_id, listener_addr.clone())
            .unwrap()
            .await
            .unwrap();

        // check connection metadata
        assert_eq!(conn.metadata.remote_peer_id, listener_peer_id);
        assert_eq!(conn.metadata.addr, listener_addr);
        assert_eq!(conn.metadata.origin, ConnectionOrigin::Outbound);
        assert_eq!(
            conn.metadata.messaging_protocol,
            MessagingProtocolVersion::V1
        );
        assert_eq!(conn.metadata.application_protocols, supported_protocols);

        // test the socket works
        let msg = write_read_msg(&mut conn.socket, b"barbaz").await;
        assert_eq!(&msg, b"foobar".as_ref());
        conn.socket.close().await.unwrap();

        // Dial again as an "untrusted" dialer

        // dial listener
        let mut conn = dialer_transport
            .dial(listener_peer_id, listener_addr.clone())
            .unwrap()
            .await
            .unwrap();

        // check connection metadata
        assert_eq!(conn.metadata.remote_peer_id, listener_peer_id);
        assert_eq!(conn.metadata.addr, listener_addr);
        assert_eq!(conn.metadata.origin, ConnectionOrigin::Outbound);
        assert_eq!(
            conn.metadata.messaging_protocol,
            MessagingProtocolVersion::V1
        );
        assert_eq!(conn.metadata.application_protocols, supported_protocols);

        // test the socket works
        let msg = write_read_msg(&mut conn.socket, b"barbaz").await;
        assert_eq!(&msg, b"foobar".as_ref());
        conn.socket.close().await.unwrap();
    };

    rt.block_on(future::join(listener_task, dialer_task));
}

////////////////////////////////////////
// AptosNetTransport<MemoryTransport> //
////////////////////////////////////////

#[test]
fn test_memory_transport_mutual_auth() {
    test_transport_success(
        memory::MemoryTransport,
        Auth::Mutual,
        "/memory/0",
        expect_memory_noise_addr,
    );
}

#[test]
fn test_memory_transport_server_only_auth() {
    test_transport_success(
        memory::MemoryTransport,
        Auth::ServerOnly,
        "/memory/0",
        expect_memory_noise_addr,
    );
}

#[test]
fn test_memory_transport_rejects_unauthed_dialer() {
    test_transport_rejects_unauthed_dialer(
        memory::MemoryTransport,
        "/memory/0",
        expect_memory_noise_addr,
    );
}

#[test]
fn test_memory_transport_maybe_mutual() {
    test_transport_maybe_mutual(
        memory::MemoryTransport,
        "/memory/0",
        expect_memory_noise_addr,
    );
}

/////////////////////////////////////
// AptosNetTransport<TcpTransport> //
/////////////////////////////////////

#[test]
fn test_tcp_transport_mutual_auth() {
    test_transport_success(
        APTOS_TCP_TRANSPORT.clone(),
        Auth::Mutual,
        "/ip4/127.0.0.1/tcp/0",
        expect_ip4_tcp_noise_addr,
    );
}

#[test]
fn test_tcp_transport_server_only_auth() {
    test_transport_success(
        APTOS_TCP_TRANSPORT.clone(),
        Auth::ServerOnly,
        "/ip4/127.0.0.1/tcp/0",
        expect_ip4_tcp_noise_addr,
    );
}

#[test]
fn test_tcp_transport_rejects_unauthed_dialer() {
    test_transport_rejects_unauthed_dialer(
        APTOS_TCP_TRANSPORT.clone(),
        "/ip4/127.0.0.1/tcp/0",
        expect_ip4_tcp_noise_addr,
    );
}
