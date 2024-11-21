// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod iterators;
pub(crate) mod truncation_helper;

use crate::schema::db_metadata::{DbMetadataKey, DbMetadataSchema};
use aptos_schemadb::{SchemaBatch, DB};
use aptos_storage_interface::Result;
use aptos_types::{state_store::NUM_STATE_SHARDS, transaction::Version};
use arr_macro::arr;

pub(crate) type ShardedStateKvSchemaBatch = [SchemaBatch; NUM_STATE_SHARDS];

pub(crate) fn get_progress(db: &DB, progress_key: &DbMetadataKey) -> Result<Option<Version>> {
    Ok(db
        .get::<DbMetadataSchema>(progress_key)?
        .map(|v| v.expect_version()))
}

pub(crate) fn new_sharded_kv_schema_batch() -> ShardedStateKvSchemaBatch {
    arr![SchemaBatch::new(); 16]
}
