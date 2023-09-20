// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{counters::TIMER, move_vm_ext::AptosMoveResolver};
use aptos_crypto::HashValue;
use aptos_framework::natives::code::PackageRegistry;
use aptos_infallible::RwLock;
use aptos_metrics_core::TimerHelper;
use aptos_types::on_chain_config::OnChainConfig;
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::StatusCode,
};
use move_vm_runtime::{config::VMConfig, move_vm::MoveVM, native_functions::NativeFunction};
use once_cell::sync::Lazy;
use std::{collections::HashMap, hash::Hash};

const WARM_VM_CACHE_SIZE: usize = 8;

pub(crate) struct WarmVmCache {
    cache: RwLock<HashMap<WarmVmId, MoveVM>>,
}

static WARM_VM_CACHE: Lazy<WarmVmCache> = Lazy::new(|| WarmVmCache {
    cache: RwLock::new(HashMap::new()),
});

impl WarmVmCache {
    pub(crate) fn get_warm_vm(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
        vm_config: VMConfig,
        resolver: &impl AptosMoveResolver,
    ) -> VMResult<MoveVM> {
        WARM_VM_CACHE.get(natives, vm_config, resolver)
    }

    fn get(
        &self,
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
        vm_config: VMConfig,
        resolver: &impl AptosMoveResolver,
    ) -> VMResult<MoveVM> {
        let id = {
            let _timer = TIMER.timer_with(&["get_warm_vm_id"]);
            WarmVmId {
                vm_config_id: VmConfigId::new(&vm_config),
                framework_id: FrameworkId::new(resolver)?,
            }
        };

        if let Some(vm) = self.cache.read().get(&id) {
            let _timer = TIMER.timer_with(&["warm_vm_cache_hit"]);
            return Ok(vm.clone());
        }

        {
            let _timer = TIMER.timer_with(&["warm_vm_cache_miss"]);
            let mut cache_locked = self.cache.write();
            if let Some(vm) = cache_locked.get(&id) {
                // Another thread has loaded it
                return Ok(vm.clone());
            }

            let vm = MoveVM::new_with_config(natives, vm_config)?;
            Self::warm_vm_up(&vm, resolver);

            // Not using LruCache because its `::get()` requires &mut self
            if cache_locked.len() >= WARM_VM_CACHE_SIZE {
                cache_locked.clear();
            }
            cache_locked.insert(id, vm.clone());
            Ok(vm)
        }
    }

    fn warm_vm_up(vm: &MoveVM, resolver: &impl AptosMoveResolver) {
        let _timer = TIMER.timer_with(&["vm_warm_up"]);

        // Loading `0x1::account` and its transitive dependency into the code cache.
        //
        // This should give us a warm VM to avoid the overhead of VM cold start.
        // Result of this load could be omitted as this is a best effort approach and won't hurt if that fails.
        //
        // Loading up `0x1::account` should be sufficient as this is the most common module
        // used for prologue, epilogue and transfer functionality.
        let _ = vm.load_module(
            &ModuleId::new(CORE_CODE_ADDRESS, ident_str!("account").to_owned()),
            &resolver,
        );
    }
}

#[derive(Eq, Hash, PartialEq)]
struct VmConfigId(HashValue);

impl VmConfigId {
    fn new(vm_config: &VMConfig) -> Self {
        let bytes = bcs::to_bytes(vm_config).expect("failed to serialize VMConfig.");
        Self(HashValue::sha3_256_of(&bytes))
    }
}

#[derive(Eq, Hash, PartialEq)]
struct FrameworkId(Option<HashValue>);

impl FrameworkId {
    fn new(resolver: &impl AptosMoveResolver) -> VMResult<Self> {
        let bytes = {
            let _timer = TIMER.timer_with(&["fetch_pkgreg"]);
            resolver.fetch_config(PackageRegistry::access_path().unwrap())
        };

        let core_package_registry = {
            let _timer = TIMER.timer_with(&["deserialize_pkgreg"]);
            bytes
                .as_ref()
                .map(|bytes| PackageRegistry::deserialize_into_config(bytes))
                .transpose()
                .map_err(|err| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("Failed to deserialize PackageRegistry: {}", err))
                        .finish(Location::Undefined)
                })?
        };

        {
            let _timer = TIMER.timer_with(&["ensure_no_ext_deps"]);
            core_package_registry
                .as_ref()
                .map(Self::ensure_no_external_dependency)
                .transpose()?;
        }

        Ok(Self(
            bytes.as_ref().map(|bytes| HashValue::sha3_256_of(bytes)),
        ))
    }

    fn ensure_no_external_dependency(core_package_registry: &PackageRegistry) -> VMResult<()> {
        for package in &core_package_registry.packages {
            for dep in &package.deps {
                if dep.account != CORE_CODE_ADDRESS {
                    return Err(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message("External dependency found in core packages.".to_string())
                            .finish(Location::Undefined),
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Eq, Hash, PartialEq)]
struct WarmVmId {
    vm_config_id: VmConfigId,
    framework_id: FrameworkId,
}
