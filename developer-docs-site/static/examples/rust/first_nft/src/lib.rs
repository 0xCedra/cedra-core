// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use first_transaction::{Account, RestClient};
use hex;
use reqwest;
use serde_json::Value;
pub struct NftClient {
    url: String,
    pub rest_client: RestClient,
}
const NUMBER_MAX: u64 = 9007199254740991;
impl NftClient {
    /// Represents an account as well as the private, public key-pair for the Aptos blockchain.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            rest_client: RestClient::new(url.to_string()),
        }
    }
    pub fn submit_transaction_helper(&self, account: &mut Account, payload: Value) {
        let hash = self
            .rest_client
            .execution_transaction_with_payload(account, payload);
        self.rest_client.wait_for_transaction(hash.as_str())
    }
    pub fn create_collection(
        &self,
        account: &mut Account,
        name: &str,
        uri: &str,
        description: &str,
    ) {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x3::token::create_collection_script",
            "type_arguments": [],
            "arguments": [
                hex::encode(name.as_bytes()),
                hex::encode(description.as_bytes()),
                hex::encode(uri.as_bytes()),
                NUMBER_MAX.to_string().as_str(),
                [false, false, false]
            ]
        });
        self.submit_transaction_helper(account, payload)
    }

    pub fn create_token(
        &self,
        account: &mut Account,
        collection_name: &str,
        name: &str,
        description: &str,
        supply: i32,
        uri: &str,
    ) {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x3::token::create_token_script",
            "type_arguments": [],
            "arguments": [
                hex::encode(collection_name.as_bytes()),
                hex::encode(name.as_bytes()),
                hex::encode(description.as_bytes()),
                supply.to_string().as_str(),
                NUMBER_MAX.to_string().as_str(),
                hex::encode(uri.as_bytes()),
                account.address(),
                "0",
                "0",
                [false, false, false, false, false],
                [],
                [],
                []
            ]
        });
        self.submit_transaction_helper(account, payload)
    }
    pub fn offer_token(
        &self,
        account: &mut Account,
        receiver: &str,
        creator: &str,
        collection_name: &str,
        token_name: &str,
        amount: i32,
    ) {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x3::token_transfers::offer_script",
            "type_arguments": [],
            "arguments": [
                receiver,
                creator,
                hex::encode(collection_name.as_bytes()),
                hex::encode(token_name.as_bytes()),
                amount.to_string().as_str(),
                "0"
            ]
        });
        self.submit_transaction_helper(account, payload)
    }
    pub fn claim_token(
        &self,
        account: &mut Account,
        sender: &str,
        creator: &str,
        collection_name: &str,
        token_name: &str,
    ) {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x3::token_transfers::claim_script",
            "type_arguments": [],
            "arguments": [
                sender,
                creator,
                hex::encode(collection_name.as_bytes()),
                hex::encode(token_name.as_bytes()),
                "0"
            ]
        });
        self.submit_transaction_helper(account, payload)
    }
    pub fn cancel_token_offer(
        &self,
        account: &mut Account,
        receiver: &str,
        creator: &str,
        token_creation_num: i32,
    ) {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function":"0x3::token_transfers::cancel_offer_script",
            "type_arguments": [],
            "arguments": [
                receiver,
                creator,
                token_creation_num.to_string().as_str(),
            ]
        });
        self.submit_transaction_helper(account, payload)
    }

    pub fn get_table_item(
        &self,
        handle: &str,
        key_type: &str,
        value_type: &str,
        key: Value,
    ) -> Value {
        let res = reqwest::blocking::Client::new()
            .post(format!("{}/tables/{}/item", self.url, handle))
            .json(&serde_json::json!({
                "key_type": key_type,
                "value_type": value_type,
                "key": key
            }))
            .send()
            .unwrap();
        res.json().unwrap()
    }
    pub fn get_collection(&self, creator: &str, collection_name: &str) -> Value {
        let collection = &self
            .rest_client
            .account_resource(creator, "0x3::token::Collections")
            .unwrap()["data"]["collection_data"]["handle"];
        match collection {
            Value::String(s) => self.get_table_item(
                s.as_str(),
                "0x1::string::String",
                "0x3::token::CollectionData",
                Value::String(hex::encode(collection_name.as_bytes())),
            ),
            _ => panic!("get_collection:error"),
        }
    }
    pub fn get_token_balance(
        &self,
        owner: &str,
        creator: &str,
        collection_name: &str,
        token_name: &str,
    ) -> Value {
        let token_store = &self
            .rest_client
            .account_resource(owner, "0x3::token::TokenStore")
            .unwrap()["data"]["tokens"]["handle"];
        let token_id = serde_json::json!({
            "token_data_id":{
                "creator": creator,
                "collection": hex::encode(collection_name),
                "name": hex::encode(token_name)
            },
            "property_version": "0",
        });
        match token_store {
            Value::String(s) => {
                self.get_table_item(s, "0x3::token::TokenId", "0x3::token::Token", token_id)
                    ["amount"]
                    .clone()
            }
            _ => panic!("get_token_balance:error"),
        }
    }
    pub fn get_token_data(&self, creator: &str, collection_name: &str, token_name: &str) -> Value {
        let token_data = &self
            .rest_client
            .account_resource(creator, "0x3::token::Collections")
            .unwrap()["data"]["token_data"]["handle"];
        let token_data_id = serde_json::json!({
            "creator": creator,
            "collection": hex::encode(collection_name),
            "name": hex::encode(token_name),
        });
        match token_data {
            Value::String(data) => self
                .get_table_item(
                    data,
                    "0x3::token::TokenDataId",
                    "0x3::token::TokenData",
                    token_data_id,
                )
                .clone(),
            _ => panic!("get_token_data:error"),
        }
    }
}
