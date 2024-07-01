// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{session::respawned_session::RespawnedSession, AptosMoveResolver, SessionId},
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOpSize,
};
use aptos_vm_types::{
    change_set::{ChangeSetInterface, VMChangeSet, WriteOpInfo},
    module_write_set::ModuleWriteSet,
    resolver::ExecutorView,
    storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_binary_format::errors::PartialVMResult;
use move_core_types::vm_status::VMStatus;

pub struct UserSessionChangeSet {
    change_set: VMChangeSet,
    module_write_set: ModuleWriteSet,
}

impl UserSessionChangeSet {
    pub(crate) fn unpack(self) -> (VMChangeSet, ModuleWriteSet) {
        (self.change_set, self.module_write_set)
    }
}

impl ChangeSetInterface for UserSessionChangeSet {
    fn num_write_ops(&self) -> usize {
        self.change_set.num_write_ops() + self.module_write_set.num_write_ops()
    }

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.change_set
            .write_set_size_iter()
            .chain(self.module_write_set.write_set_size_iter())
    }

    fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo>> {
        self.change_set
            .write_op_info_iter_mut(executor_view)
            .chain(self.module_write_set.write_op_info_iter_mut(executor_view))
    }

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.change_set.events_iter()
    }
}

#[derive(Deref, DerefMut)]
pub struct UserSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    pub session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> UserSession<'r, 'l> {
    pub fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        prologue_change_set: VMChangeSet,
    ) -> Self {
        let session_id = SessionId::txn_meta(txn_meta);

        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            prologue_change_set,
            Some(txn_meta.as_user_transaction_context()),
        );

        Self { session }
    }

    pub fn legacy_inherit_prologue_session(prologue_session: RespawnedSession<'r, 'l>) -> Self {
        Self {
            session: prologue_session,
        }
    }

    pub fn finish(
        self,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        let Self { session } = self;
        let (change_set, module_write_set) =
            session.finish_with_squashed_change_set(change_set_configs, false)?;
        let user_session_change_set = UserSessionChangeSet {
            change_set,
            module_write_set,
        };

        change_set_configs.check_change_set(&user_session_change_set)?;
        Ok(user_session_change_set)
    }
}
