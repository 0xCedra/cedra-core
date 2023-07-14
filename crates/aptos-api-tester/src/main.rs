// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use aptos_api_types::U64;
use aptos_cached_packages::aptos_stdlib::EntryFunctionCall;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_rest_client::error::RestError;
use aptos_rest_client::{Account, Client, FaucetClient};
use aptos_sdk::bcs;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::token_client::{
    build_and_submit_transaction, CollectionData, CollectionMutabilityConfig, RoyaltyOptions,
    TokenClient, TokenData, TokenMutabilityConfig, TransactionOptions,
};
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use url::Url;

// network urls
static DEVNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.devnet.aptoslabs.com").unwrap());
static DEVNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.devnet.aptoslabs.com").unwrap());
static TESTNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.testnet.aptoslabs.com").unwrap());
static TESTNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.testnet.aptoslabs.com").unwrap());

#[derive(Debug)]
enum TestResult {
    Success,
}

#[derive(Debug)]
enum TestFailure {
    Fail(&'static str),
    Error(anyhow::Error),
}

impl From<RestError> for TestFailure {
    fn from(e: RestError) -> TestFailure {
        TestFailure::Error(e.into())
    }
}

impl From<anyhow::Error> for TestFailure {
    fn from(e: anyhow::Error) -> TestFailure {
        TestFailure::Error(e)
    }
}

async fn handle_result<Fut: Future<Output = Result<TestResult, TestFailure>>>(
    fut: Fut,
) -> Result<TestResult, TestFailure> {
    let result = fut.await;
    match &result {
        Ok(success) => println!("{:?}", success),
        Err(failure) => println!("{:?}", failure),
    }
    result
}

/// Tests new account creation. Checks that:
///   - account data exists
///   - account balance reflects funded amount
async fn test_newaccount(
    client: &Client,
    account: &LocalAccount,
    amount_funded: u64,
) -> Result<TestResult, TestFailure> {
    // ask for account data
    let response = client.get_account(account.address()).await?;

    // check account data
    let expected_account = Account {
        authentication_key: account.authentication_key(),
        sequence_number: account.sequence_number(),
    };
    let actual_account = response.inner();

    if &expected_account != actual_account {
        return Err(TestFailure::Fail("wrong account data"));
    }

    // check account balance
    let expected_balance = U64(amount_funded);
    let actual_balance = client
        .get_account_balance(account.address())
        .await?
        .inner()
        .coin
        .value;

    if expected_balance != actual_balance {
        return Err(TestFailure::Fail("wrong balance"));
    }

    Ok(TestResult::Success)
}

/// Tests coin transfer. Checks that:
///   - receiver balance reflects transferred amount
///   - receiver balance shows correct amount at the previous version
async fn test_cointransfer(
    client: &Client,
    coin_client: &CoinClient<'_>,
    account: &mut LocalAccount,
    receiver: AccountAddress,
    amount: u64,
) -> Result<TestResult, TestFailure> {
    // get starting balance
    let starting_receiver_balance = u64::from(
        client
            .get_account_balance(receiver)
            .await?
            .inner()
            .coin
            .value,
    );

    // transfer coins to static account
    let pending_txn = coin_client
        .transfer(account, receiver, amount, None)
        .await?;
    let response = client.wait_for_transaction(&pending_txn).await?;

    // check receiver balance
    let expected_receiver_balance = U64(starting_receiver_balance + amount);
    let actual_receiver_balance = client
        .get_account_balance(receiver)
        .await?
        .inner()
        .coin
        .value;

    if expected_receiver_balance != actual_receiver_balance {
        return Err(TestFailure::Fail("wrong balance after coin transfer"));
    }

    // check account balance with a lower version number
    let version = match response.inner().version() {
        Some(version) => version,
        _ => {
            return Err(TestFailure::Error(anyhow!(
                "transaction did not return version"
            )))
        },
    };

    let expected_balance_at_version = U64(starting_receiver_balance);
    let actual_balance_at_version = client
        .get_account_balance_at_version(receiver, version - 1)
        .await?
        .inner()
        .coin
        .value;

    if expected_balance_at_version != actual_balance_at_version {
        return Err(TestFailure::Fail(
            "wrong balance at version before the coin transfer",
        ));
    }

    Ok(TestResult::Success)
}

/// Tests nft transfer. Checks that:
///   - collection data exists
///   - token data exists
///   - token balance reflects transferred amount
async fn test_mintnft(
    client: &Client,
    token_client: &TokenClient<'_>,
    account: &mut LocalAccount,
    receiver: &mut LocalAccount,
) -> Result<TestResult, TestFailure> {
    // create collection
    let collection_name = "test collection".to_string();
    let collection_description = "collection description".to_string();
    let collection_uri = "collection uri".to_string();
    let collection_maximum = 1000;

    let pending_txn = token_client
        .create_collection(
            account,
            &collection_name,
            &collection_description,
            &collection_uri,
            collection_maximum,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // create token
    let token_name = "test token".to_string();
    let token_description = "token description".to_string();
    let token_uri = "token uri".to_string();
    let token_maximum = 1000;
    let token_supply = 10;

    let pending_txn = token_client
        .create_token(
            account,
            &collection_name,
            &token_name,
            &token_description,
            token_supply,
            &token_uri,
            token_maximum,
            None,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // check collection metadata
    let expected_collection_data = CollectionData {
        name: collection_name.clone(),
        description: collection_description,
        uri: collection_uri,
        maximum: U64(collection_maximum),
        mutability_config: CollectionMutabilityConfig {
            description: false,
            maximum: false,
            uri: false,
        },
    };
    let actual_collection_data = token_client
        .get_collection_data(account.address(), &collection_name)
        .await?;

    if expected_collection_data != actual_collection_data {
        return Err(TestFailure::Fail("wrong collection data"));
    }

    // check token metadata
    let expected_token_data = TokenData {
        name: token_name.clone(),
        description: token_description,
        uri: token_uri,
        maximum: U64(token_maximum),
        mutability_config: TokenMutabilityConfig {
            description: false,
            maximum: false,
            properties: false,
            royalty: false,
            uri: false,
        },
        supply: U64(token_supply),
        royalty: RoyaltyOptions {
            payee_address: account.address(),
            royalty_points_denominator: U64(0),
            royalty_points_numerator: U64(0),
        },
        largest_property_version: U64(0),
    };
    let actual_token_data = token_client
        .get_token_data(account.address(), &collection_name, &token_name)
        .await?;

    if expected_token_data != actual_token_data {
        return Err(TestFailure::Fail("wrong token data"));
    }

    // offer token
    let pending_txn = token_client
        .offer_token(
            account,
            receiver.address(),
            account.address(),
            &collection_name,
            &token_name,
            2,
            None,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // check token balance for the sender
    let expected_sender_token_balance = U64(8);
    let actual_sender_token_balance = token_client
        .get_token(
            account.address(),
            account.address(),
            &collection_name,
            &token_name,
        )
        .await?
        .amount;

    if expected_sender_token_balance != actual_sender_token_balance {
        return Err(TestFailure::Fail("wrong token balance"));
    }

    // check that token store isn't initialized for the receiver
    match token_client
        .get_token(
            receiver.address(),
            account.address(),
            &collection_name,
            &token_name,
        )
        .await
    {
        Ok(_) => {
            return Err(TestFailure::Fail(
                "found tokens for receiver when shouldn't",
            ))
        },
        Err(_) => {},
    }

    // claim token
    let pending_txn = token_client
        .claim_token(
            receiver,
            account.address(),
            account.address(),
            &collection_name,
            &token_name,
            None,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // check token balance for the receiver
    let expected_receiver_token_balance = U64(2);
    let actual_receiver_token_balance = token_client
        .get_token(
            receiver.address(),
            account.address(),
            &collection_name,
            &token_name,
        )
        .await?
        .amount;

    if expected_receiver_token_balance != actual_receiver_token_balance {
        return Err(TestFailure::Fail("wrong token balance"));
    }

    Ok(TestResult::Success)
}

async fn test_module(
    client: &Client,
    account: &mut LocalAccount,
) -> Result<TestResult, TestFailure> {
    // get file to compile
    let move_dir = PathBuf::from("/Users/ngk/Documents/aptos-core/aptos-move/move-examples/hello_blockchain");

    // insert address
    let mut named_addresses: BTreeMap<String, AccountAddress> = BTreeMap::new();
    named_addresses.insert("hello_blockchain".to_string(), account.address());

    // build options
    let mut options: BuildOptions = BuildOptions::default();
    options.named_addresses = named_addresses;

    // build module
    let package = BuiltPackage::build(move_dir, options)?;
    let blobs = package.extract_code();
    let metadata = package.extract_metadata()?;

    // create payload
    let payload = EntryFunctionCall::CodePublishPackageTxn {
        metadata_serialized: bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
        code: blobs,
    }
    .encode();

    // create and submit transaction
    let pending_txn =
        build_and_submit_transaction(&client, account, payload, TransactionOptions::default())
            .await?;
    client.wait_for_transaction(&pending_txn).await?;

    Ok(TestResult::Success)
}

async fn test_flows(client: Client, faucet_client: FaucetClient) -> Result<()> {
    // create clients
    let coin_client = CoinClient::new(&client);
    let token_client = TokenClient::new(&client);

    // create and fund account for tests
    let mut giray = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(giray.address(), 100_000_000).await?;
    println!("{:?}", giray.address());

    let mut giray2 = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(giray2.address(), 100_000_000).await?;
    println!("{:?}", giray2.address());

    // Test new account creation and funding
    // this test is critical to pass for the next tests
    match handle_result(test_newaccount(&client, &giray, 100_000_000)).await {
        Err(_) => return Err(anyhow!("returning early because new account test failed")),
        _ => {},
    }

    // Flow 1: Coin transfer
    let _ = handle_result(test_cointransfer(
        &client,
        &coin_client,
        &mut giray,
        giray2.address(),
        1_000,
    ))
    .await;

    // Flow 2: NFT transfer
    let _ = handle_result(test_mintnft(
        &client,
        &token_client,
        &mut giray,
        &mut giray2,
    ))
    .await;

    // Flow 3: NFT transfer
    let _ = handle_result(test_module(&client, &mut giray)).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // test flows on testnet
    println!("testing testnet...");
    let _ = test_flows(
        Client::new(TESTNET_NODE_URL.clone()),
        FaucetClient::new(TESTNET_FAUCET_URL.clone(), TESTNET_NODE_URL.clone()),
    )
    .await;

    // test flows on devnet
    println!("testing devnet...");
    let _ = test_flows(
        Client::new(DEVNET_NODE_URL.clone()),
        FaucetClient::new(DEVNET_FAUCET_URL.clone(), DEVNET_NODE_URL.clone()),
    )
    .await;

    Ok(())
}
