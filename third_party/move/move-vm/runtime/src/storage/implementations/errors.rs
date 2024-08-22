// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! module_storage_error {
    ($addr:ident, $name:ident, $err:ident) => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::STORAGE_ERROR,
        )
        .with_message(format!(
            "Unexpected storage error for module {}::{}: {:?}",
            $addr, $name, $err
        ))
    };
}

#[macro_export]
macro_rules! module_linker_error {
    ($addr:ident, $name:ident) => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::LINKER_ERROR,
        )
        .with_message(format!("Module {}::{} does not exist", $addr, $name))
    };
}

#[macro_export]
macro_rules! module_cyclic_dependency_error {
    ($addr:ident, $name:ident) => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::CYCLIC_MODULE_DEPENDENCY,
        )
        .with_message(format!(
            "Module {}::{} forms a cyclic dependency",
            $addr, $name
        ))
    };
}
