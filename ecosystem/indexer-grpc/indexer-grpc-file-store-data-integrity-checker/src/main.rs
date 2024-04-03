// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_file_store_data_integrity_checker::IndexerGrpcFileStoreDataIntegrityCheckerConfig;
use aptos_indexer_grpc_server_framework::ServerArgs;
use clap::Parser;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.run::<IndexerGrpcFileStoreDataIntegrityCheckerConfig>()
        .await
        .expect("File store data integrity checker failed to run");
    Ok(())
}
