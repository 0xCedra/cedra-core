// Copyright © Aptos Foundation

use anyhow::Result;
use aptos_channels::aptos_channel;
use aptos_logger::info;
use aptos_types::jwks::{jwk::JWK, Issuer};
use futures::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
#[cfg(feature = "smoke-test")]
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{sync::oneshot, task::JoinHandle, time::MissedTickBehavior};

#[derive(Serialize, Deserialize)]
struct OpenIDConfiguration {
    issuer: String,
    jwks_uri: String,
}

#[derive(Serialize, Deserialize)]
struct JWKsResponse {
    keys: Vec<serde_json::Value>,
}

#[cfg(feature = "smoke-test")]
pub async fn fetch_jwks(my_addr: AccountAddress, config_url: Vec<u8>) -> Result<Vec<JWK>> {
    let maybe_url = String::from_utf8(config_url);
    let config_url = maybe_url?;
    let client = reqwest::Client::new();
    let JWKsResponse { keys } = client
        .get(config_url.as_str())
        .header(header::COOKIE, my_addr.to_hex())
        .send()
        .await?
        .json()
        .await?;
    let jwks = keys.into_iter().map(JWK::from).collect();
    Ok(jwks)
}

#[cfg(not(feature = "smoke-test"))]
pub async fn fetch_jwks(_my_addr: AccountAddress, config_url: Vec<u8>) -> Result<Vec<JWK>> {
    let maybe_url = String::from_utf8(config_url);
    let config_url = maybe_url?;
    let client = reqwest::Client::new();
    let OpenIDConfiguration { jwks_uri, .. } =
        client.get(config_url.as_str()).send().await?.json().await?;
    let JWKsResponse { keys } = client.get(jwks_uri.as_str()).send().await?.json().await?;
    let jwks = keys.into_iter().map(JWK::from).collect();
    Ok(jwks)
}

pub struct JWKObserver {
    close_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

impl JWKObserver {
    pub fn spawn(
        my_addr: AccountAddress,
        issuer: Issuer,
        config_url: Vec<u8>,
        fetch_interval: Duration,
        observation_tx: aptos_channel::Sender<(), (Issuer, Vec<JWK>)>,
    ) -> Self {
        let (close_tx, close_rx) = oneshot::channel();
        let join_handle = tokio::spawn(Self::thread_main(
            fetch_interval,
            my_addr,
            issuer.clone(),
            config_url.clone(),
            observation_tx,
            close_rx,
        ));
        info!(
            "[JWK] observer spawned, issuer={:?}, config_url={:?}",
            String::from_utf8(issuer),
            String::from_utf8(config_url)
        );
        Self {
            close_tx,
            join_handle,
        }
    }

    async fn thread_main(
        fetch_interval: Duration,
        my_addr: AccountAddress,
        issuer: Issuer,
        open_id_config_url: Vec<u8>,
        observation_tx: aptos_channel::Sender<(), (Issuer, Vec<JWK>)>,
        close_rx: oneshot::Receiver<()>,
    ) {
        let mut interval = tokio::time::interval(fetch_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut close_rx = close_rx.into_stream();
        loop {
            tokio::select! {
                _ = interval.tick().fuse() => {
                    let result = fetch_jwks(my_addr, open_id_config_url.clone()).await;
                    if let Ok(jwks) = result {
                        let _ = observation_tx.push((), (issuer.clone(), jwks));
                    }
                },
                _ = close_rx.select_next_some() => {
                    break;
                }
            }
        }
    }

    pub async fn shutdown(self) {
        let Self {
            close_tx,
            join_handle,
        } = self;
        let _ = close_tx.send(());
        let _ = join_handle.await;
    }
}

#[tokio::test]
async fn test_fetch_jwks() {
    let jwks = fetch_jwks(
        AccountAddress::ZERO,
        "https://www.facebook.com/.well-known/openid-configuration/"
            .as_bytes()
            .to_vec(),
    )
    .await
    .unwrap();
    println!("{:?}", jwks);
}
