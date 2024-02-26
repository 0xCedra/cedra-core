// Copyright © Aptos Foundation

use crate::vrf_keys::VRF_SCHEME0_SK;
use anyhow::{anyhow, bail, ensure};
use aptos_oidb_pepper_common::{
    jwt::Claims, sha3_256, vrf::{self, VRF}, PepperInput, PepperRequest, PepperResponse
};
use aptos_types::{
    oidb::{Configuration, OpenIdSig},
    transaction::authenticator::EphemeralPublicKey,
};
use jsonwebtoken::{Algorithm::RS256, Validation};
use once_cell::sync::Lazy;
use std::collections::HashSet;

pub mod about;
pub mod jwk;
pub mod vrf_keys;

pub type Issuer = String;
pub type KeyID = String;

/// The core processing logic of this pepper service.
pub fn process(request: PepperRequest) -> anyhow::Result<PepperResponse> {
    let pepper_key_hex_string = process_v0(request)?;
    let pepper = sha3_256(&hex::decode(pepper_key_hex_string.clone())?);
    let pepper_hex_string = hex::encode(pepper[..31].to_vec());
    Ok(PepperResponse{ pepper_key_hex_string, pepper_hex_string })
}

fn process_v0(request: PepperRequest) -> anyhow::Result<String> {
    let PepperRequest {
        jwt_b64: jwt,
        overriding_aud,
        epk_hex_string,
        epk_expiry_time_secs,
        epk_blinder_hex_string,
        uid_key,
    } = request;

    let claims = aptos_oidb_pepper_common::jwt::parse(jwt.as_str())
        .map_err(|e| anyhow!("JWT decoding error: {e}"))?;

    let actual_uid_key = if let Some(uid_key) = uid_key.as_ref() {
        uid_key
    } else {
        "sub"
    };

    let uid_val = if actual_uid_key == "email" {
        claims
            .claims
            .email
            .clone()
            .ok_or_else(|| anyhow!("`email` required but not found in jwt"))?
    } else if actual_uid_key == "sub" {
        claims.claims.sub.clone()
    } else {
        bail!("unsupported uid key: {}", actual_uid_key)
    };

    let blinder = hex::decode(epk_blinder_hex_string)
        .map_err(|e| anyhow!("blinder unhexlification error: {e}"))?;
    let epk_bytes = hex::decode(epk_hex_string)
        .map_err(|e| anyhow!("epk unhexlification error: {e}"))?;
    let epk = bcs::from_bytes::<EphemeralPublicKey>(&epk_bytes)
        .map_err(|e| anyhow!("epk bcs deserialization error: {e}"))?;
    let recalculated_nonce = OpenIdSig::reconstruct_oauth_nonce(
        blinder.as_slice(),
epk_expiry_time_secs,
        &epk,
        &Configuration::new_for_devnet(),
    )
    .map_err(|e| anyhow!("nonce reconstruction error: {e}"))?;

    ensure!(
        claims.claims.nonce == recalculated_nonce,
        "with nonce mismatch"
    );

    let key_id = claims
        .header
        .kid
        .ok_or_else(|| anyhow!("missing kid in JWT"))?;

    let sig_pub_key = jwk::cached_decoding_key(&claims.claims.iss, &key_id)?;
    let mut validation_with_sig_verification = Validation::new(RS256);
    validation_with_sig_verification.validate_exp = false; // Don't validate the exp time
    let _claims = jsonwebtoken::decode::<Claims>(
        jwt.as_str(),
        sig_pub_key.as_ref(),
        &validation_with_sig_verification,
    ) // Signature verification happens here.
    .map_err(|e| anyhow!("JWT signature verification failed: {e}"))?;

    // Decide the client_id in the input.
    let actual_aud = if ACCOUNT_DISCOVERY_CLIENTS.contains(&claims.claims.aud) {
        if let Some(aud) = overriding_aud.as_ref() {
            aud
        } else {
            &claims.claims.aud
        }
    } else {
        &claims.claims.aud
    };

    let input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_owned(),
        uid_val,
        aud: actual_aud.clone(),
    };
    let input_bytes = bcs::to_bytes(&input).unwrap();
    let (pepper, vuf_proof) = vrf::scheme0::Scheme0::eval(&VRF_SCHEME0_SK, &input_bytes)?;
    ensure!(vuf_proof.is_empty(), "internal proof error");
    let pepper_hexlified = hex::encode(pepper);
    Ok(pepper_hexlified)
}

/// The set of the privileged clients.
///
/// TODO: should be loaded from env/an external service.
pub static ACCOUNT_DISCOVERY_CLIENTS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("407408718192.apps.googleusercontent.com".to_string()); // Google OAuth 2.0 Playground
    set
});
