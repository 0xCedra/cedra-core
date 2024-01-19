// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{AptosMoveResolver, SessionExt};
use anyhow::{Context, Result};
use aptos_types::{
    jwks::{jwk::JWK, PatchedJWKs},
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    transaction::SignedTransaction,
    vm_status::{StatusCode, VMStatus},
    zkid::{ZkpOrOpenIdSig, MAX_ZK_ID_AUTHENTICATORS_ALLOWED},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use move_core_types::{language_storage::CORE_CODE_ADDRESS, move_resource::MoveStructType};

fn get_current_time_onchain(resolver: &impl AptosMoveResolver) -> Result<CurrentTimeMicroseconds, VMStatus> {
    CurrentTimeMicroseconds::fetch_config(resolver).ok_or_else(VMStatus::error(StatusCode::VALUE_DESERIALIZATION_ERROR, Some("could not fetch CurrentTimeMicroseconds on-chain config".to_string())))
}

fn get_jwks_onchain(resolver: &impl AptosMoveResolver) -> Result<PatchedJWKs> {
    let bytes = resolver
        .get_resource(&CORE_CODE_ADDRESS, &PatchedJWKs::struct_tag())?
        .context("Failed to get jwks resource")?;
    let jwks = bcs::from_bytes::<PatchedJWKs>(&bytes)?;
    Ok(jwks)
}

pub fn validate_zkid_authenticators(
    transaction: &SignedTransaction,
    resolver: &impl AptosMoveResolver,
    _session: &mut SessionExt,
    _log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    // TODO(ZkIdGroth16Zkp): The ZKP/OpenID sig verification does not charge gas. So, we could have DoS attacks.
    let zkid_authenticators =
        aptos_types::zkid::get_zkid_authenticators(transaction).map_err(|_| {
            VMStatus::error(
                StatusCode::INVALID_SIGNATURE,
                Some("Failed to fetch zkid authenticators".to_owned()),
            )
        })?;

    if zkid_authenticators.is_empty() {
        return Ok(());
    }

    if zkid_authenticators.len() > MAX_ZK_ID_AUTHENTICATORS_ALLOWED {
        return Err(VMStatus::error(
            StatusCode::TOO_MANY_ZK_AUTHENTICATORS,
            None,
        ));
    }

    let onchain_timestamp_obj = get_current_time_onchain(resolver).map_err(|_| {
        VMStatus::error(
            StatusCode::FAILED_TO_GET_ON_CHAIN_CURRENT_TIME,
            Some("Failed to get CurrentTimeMicroseconds".to_owned()),
        )
    })?;
    // Check the expiry timestamp on all authenticators first to fail fast
    for (_, zkid_sig) in &zkid_authenticators {
        zkid_sig
            .verify_expiry(&onchain_timestamp_obj)
            .map_err(|_| {
                VMStatus::error(
                    StatusCode::EPHEMERAL_KEYPAIR_EXPIRED,
                    Some("The ephemeral keypair has expired".to_owned()),
                )
            })?;
    }

    let patched_jwks = get_jwks_onchain(resolver).map_err(|_| {
        VMStatus::error(
            StatusCode::FAILED_TO_GET_ON_CHAIN_JWKS,
            Some("Failed to get jwk map".to_owned()),
        )
    })?;
    for (zkid_pub_key, zkid_sig) in &zkid_authenticators {
        let jwt_header_parsed = zkid_sig.parse_jwt_header().map_err(|_| {
            VMStatus::error(
                StatusCode::INVALID_JWT_HEADER,
                Some("Failed to get JWT header".to_owned()),
            )
        })?;
        let jwk_move_struct = patched_jwks
            .get_jwk(&zkid_pub_key.iss, &jwt_header_parsed.kid)
            .map_err(|_| {
                VMStatus::error(
                    StatusCode::FAILED_TO_FIND_JWK,
                    Some("JWK not found".to_owned()),
                )
            })?;

        let jwk = JWK::try_from(jwk_move_struct).map_err(|_| {
            VMStatus::error(
                StatusCode::UNSUPPORTED_JWK,
                Some("Could not parse JWK".to_owned()),
            )
        })?;

        let jwt_header = &zkid_sig.jwt_header;

        match &zkid_sig.sig {
            ZkpOrOpenIdSig::Groth16Zkp(_) => {},
            ZkpOrOpenIdSig::OpenIdSig(openid_sig) => {
                match jwk {
                    JWK::RSA(rsa_jwk) => {
                        // TODO(OpenIdSig): Implement batch verification for all RSA signatures in
                        //  one TXN.
                        // Note: Individual OpenID RSA signature verification will be fast when the
                        // RSA public exponent is small (e.g., 65537). For the same TXN, batch
                        // verification of all RSA signatures will be even faster even when the
                        // exponent is the same. Across different TXNs, batch verification will be
                        // (1) more difficult to implement and (2) not very beneficial since, when
                        // it fails, bad signature identification will require re-verifying all
                        // signatures assuming an adversarial batch.
                        //
                        // We are now ready to verify the RSA signature
                        openid_sig
                            .verify_jwt_signature(rsa_jwk, jwt_header)
                            .map_err(|_| {
                                VMStatus::error(
                                    StatusCode::JWT_VERIFICATION_FAILED,
                                    Some(
                                        "RSA Signature verification failed for OpenIdSig"
                                            .to_owned(),
                                    ),
                                )
                            })?;
                    },
                    JWK::Unsupported(_) => {
                        return Err(VMStatus::error(
                            StatusCode::UNSUPPORTED_JWK,
                            Some("JWK is not supported".to_owned()),
                        ))
                    },
                }
            },
        }
    }
    Ok(())
}
