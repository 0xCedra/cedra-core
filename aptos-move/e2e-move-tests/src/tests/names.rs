// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//

use crate::{assert_success, MoveHarness};
use aptos_types::account_address::AccountAddress;
use cached_packages::aptos_names_sdk_builder;

/*
    Below values are for testing only!
    addresses: 0x0ee16f0e4b47d5972f63a642385d52d301e53716b4e1fbd637b9a91a7f1979ba, 0xe5a6fcac1fc4eeec1859d9e395d6c6bc49fa7dd29ca8681e581b0950dcec23df
    public_keys: 0xc5547463e44c3ad8ad52018f0aaf237d39e396b22815cf712493dd61cffabebf, 0xeea1decaa37eb5cdcf99262c6518053126e34283f42ad74f7b91b75fa625c6f8
    private_keys: 0x44c7eabad483e04ce6703a4518d0a74a1356b9c50a3f5cfd4a4c9285591caca6, 0x0afd9ed1d3c00ef22b78a7234f436132317d7fcc69824a16f0c651658929e7f8
    multisig_pub_key: 0xc5547463e44c3ad8ad52018f0aaf237d39e396b22815cf712493dd61cffabebfeea1decaa37eb5cdcf99262c6518053126e34283f42ad74f7b91b75fa625c6f801
    multisig_auth_key: 0x4407b9a063ac530f8b621f7d80b527a79c626791b14b51c1118763ce941b99ce
    threshold: 1/2
*/

#[test]
fn test_names_end_to_end() {
    let mut harness = MoveHarness::new();

    let user1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let user2 = harness.new_account_at(AccountAddress::from_hex_literal("0x456").unwrap());

    // Register a domain
    assert_success!(harness.run_transaction_payload(
        &user1,
        aptos_names_sdk_builder::domains_register_domain("max".to_string().into_bytes(), 2),
    ));

    // Set the name to point to user 2
    assert_success!(harness.run_transaction_payload(
        &user1,
        aptos_names_sdk_builder::domains_set_domain_address(
            "max".to_string().into_bytes(),
            *user2.address()
        ),
    ));

    // Clear the name, as user2
    assert_success!(harness.run_transaction_payload(
        &user2,
        aptos_names_sdk_builder::domains_clear_domain_address("max".to_string().into_bytes()),
    ));
}
