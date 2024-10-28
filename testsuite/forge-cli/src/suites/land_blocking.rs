// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ungrouped::mixed_emit_job;
use crate::{suites::{pfn::pfn_const_tps, realistic_environment::realistic_env_max_load_test}, TestCommand};
use aptos_forge::{success_criteria::SuccessCriteria, ForgeConfig, TransactionType};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade, framework_upgrade::FrameworkUpgrade,
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Attempts to match the test name to a land-blocking test
pub(crate) fn get_land_blocking_test(
    test_name: &str,
    duration: Duration,
    _test_cmd: &TestCommand,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "land_blocking" | "realistic_env_max_load" => {
            pfn_const_tps(duration, true, true, false, TransactionType::default(), 1024)
        },
        "compat" => compat(),
        "framework_upgrade" => framework_upgrade(),
        _ => return None, // The test name does not match a land-blocking test
    };
    Some(test)
}

pub(crate) fn compat() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(SimpleValidatorUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                SimpleValidatorUpgrade::EPOCH_DURATION_SECS.into();
        }))
}

pub(crate) fn framework_upgrade() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(FrameworkUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                FrameworkUpgrade::EPOCH_DURATION_SECS.into();
        }))
        .with_emit_job(mixed_emit_job())
}
