// Copyright © Aptos Foundation

use crate::{move_any::AsMoveAny, move_utils::as_move_value::AsMoveValue};
use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

/// Move type `0x1::jwks::UnsupportedJWK` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct UnsupportedJWK {
    pub id: Vec<u8>,
    pub payload: Vec<u8>,
}

impl UnsupportedJWK {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_test(id: &str, payload: &str) -> Self {
        Self {
            id: id.as_bytes().to_vec(),
            payload: payload.as_bytes().to_vec(),
        }
    }
}

impl TryFrom<&serde_json::Value> for UnsupportedJWK {
    type Error = anyhow::Error;

    fn try_from(json_value: &serde_json::Value) -> Result<Self, Self::Error> {
        let payload = json_value.to_string().into_bytes();
        let ret = Self {
            id: HashValue::sha3_256_of(payload.as_slice()).to_vec(),
            payload,
        };
        Ok(ret)
    }
}

impl AsMoveValue for UnsupportedJWK {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.payload.as_move_value()]))
    }
}

impl AsMoveAny for UnsupportedJWK {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwks::UnsupportedJWK";
}
