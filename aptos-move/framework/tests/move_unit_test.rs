// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{extended_checks, path_in_crate};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{account_config::CORE_CODE_ADDRESS, on_chain_config::{Features, OnChainConfig, TimedFeaturesBuilder}};
use aptos_vm::natives;
use move_cli::base::test::{run_move_unit_tests, UnitTestResult};
use move_command_line_common::{env::read_bool_env_var, testing::MOVE_COMPILER_V2};
use move_core_types::effects::{ChangeSet, Op};
use move_package::{CompilerConfig, CompilerVersion};
use move_unit_test::UnitTestingConfig;
use move_vm_runtime::native_functions::NativeFunctionTable;
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    let mut compiler_config = CompilerConfig {
        known_attributes: extended_checks::get_all_attribute_names().clone(),
        ..Default::default()
    };
    let mut build_config = move_package::BuildConfig {
        test_mode: true,
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        compiler_config: compiler_config.clone(),
        full_model_generation: true, // Run extended checks also on test code
        ..Default::default()
    };

    let mut ok = run_move_unit_tests(
        &pkg_path,
        build_config.clone(),
        // TODO(Gas): double check if this is correct
        UnitTestingConfig::default_with_bound(Some(100_000)),
        aptos_test_natives(),
        aptos_test_genesis(),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut std::io::stdout(),
    )
    .unwrap();
    if ok != UnitTestResult::Success {
        panic!("move unit tests failed")
    }
    if read_bool_env_var(MOVE_COMPILER_V2) {
        // Run test against v2 when MOVE_COMPILER_V2 is set
        compiler_config.compiler_version = Some(CompilerVersion::V2);
        build_config.compiler_config = compiler_config;
        ok = run_move_unit_tests(
            &pkg_path,
            build_config,
            UnitTestingConfig::default_with_bound(Some(100_000)),
            aptos_test_natives(),
            aptos_test_genesis(),
            /* cost_table */ None,
            /* compute_coverage */ false,
            &mut std::io::stdout(),
        )
        .unwrap();
    }
    if ok != UnitTestResult::Success {
        panic!("move unit tests failed for compiler v2")
    }
}

pub fn aptos_test_natives() -> NativeFunctionTable {
    // By side effect, configure for unit tests
    natives::configure_for_unit_test();
    extended_checks::configure_extended_checks_for_unit_test();
    // move_stdlib has the testing feature enabled to include debug native functions
    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}

pub fn aptos_test_genesis() -> ChangeSet {
    let features_value = bcs::to_bytes(&Features::default()).unwrap();

    let mut change_set = ChangeSet::new();
    // we need to initialize features to their defaults.
    change_set.add_resource_op(
        CORE_CODE_ADDRESS,
        Features::struct_tag(),
        Op::New(features_value.into()),
    ).expect("adding genesis Feature resource must succeed");

    change_set
}

#[test]
fn move_framework_unit_tests() {
    run_tests_for_pkg("aptos-framework");
}

#[test]
fn move_aptos_stdlib_unit_tests() {
    run_tests_for_pkg("aptos-stdlib");
}

#[test]
fn move_stdlib_unit_tests() {
    run_tests_for_pkg("move-stdlib");
}

#[test]
fn move_token_unit_tests() {
    run_tests_for_pkg("aptos-token");
}

#[test]
fn move_token_objects_unit_tests() {
    run_tests_for_pkg("aptos-token-objects");
}
