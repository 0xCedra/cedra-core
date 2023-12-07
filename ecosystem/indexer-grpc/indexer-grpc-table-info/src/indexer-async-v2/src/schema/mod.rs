// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines representation of AptosDB indexer data structures at physical level via schemas
//! that implement [`aptos_schemadb::schema::Schema`].
//!
//! All schemas are `pub(crate)` so not shown in rustdoc, refer to the source code to see details.

/// This file is a copy of the file storage/indexer/src/schema/mod.rs.
/// At the end of the migration to migrate table info mapping
/// from storage critical path to indexer, the other file will be removed.
pub(crate) mod indexer_metadata;
pub(crate) mod table_info;
use aptos_schemadb::ColumnFamilyName;

pub const DEFAULT_COLUMN_FAMILY_NAME: ColumnFamilyName = "default";
pub const INDEXER_METADATA_CF_NAME: ColumnFamilyName = "indexer_metadata";
pub const TABLE_INFO_CF_NAME: ColumnFamilyName = "table_info";

pub fn column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        INDEXER_METADATA_CF_NAME,
        TABLE_INFO_CF_NAME,
    ]
}
