// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

///! MoveVM and Session wrapped, to make sure Aptos natives and extensions are always installed and
///! taken care of after session finish.
mod resolver;
mod session;
mod vm;

pub use crate::move_vm_ext::{
    aggregator_extension::{
        aggregator_natives, AggregatorChange, AggregatorChangeSet, AggregatorID,
        NativeAggregatorContext,
    },
    resolver::MoveResolverExt,
    session::{SessionExt, SessionId, SessionOutput},
    vm::MoveVmExt,
};
