// Copyright © Aptos Foundation

use aptos_block_partitioner::{BlockPartitioner, report_sub_block_matrix, sharded_block_partitioner::ShardedBlockPartitioner, test_utils::{create_signed_p2p_transaction, generate_test_account, TestAccount}};
use clap::Parser;
use rand::rngs::OsRng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{sync::Mutex, time::Instant};
use aptos_block_partitioner::sharded_block_partitioner::counters::SHARDED_PARTITIONER_MISC_SECONDS;
use aptos_block_partitioner::simple_partitioner::{SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS, SimplePartitioner};
use aptos_logger::{error, info};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use aptos_types::transaction::Transaction;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 99000)]
    pub num_accounts: usize,

    #[clap(long, default_value_t = 100000)]
    pub block_size: usize,

    #[clap(long, default_value_t = 3)]
    pub num_blocks: usize,

    #[clap(long, default_value_t = 112)]
    pub num_shards: usize,
}

fn main() {
    println!("Starting the block partitioning benchmark");
    let args = Args::parse();
    let num_accounts = args.num_accounts;
    println!("Creating {} accounts", num_accounts);
    let accounts: Vec<Mutex<TestAccount>> = (0..num_accounts)
        .into_par_iter()
        .map(|_i| Mutex::new(generate_test_account()))
        .collect();
    println!("Created {} accounts", num_accounts);

    // let partitioner = SimplePartitioner{};
    let partitioner = ShardedBlockPartitioner::new(args.num_shards);
    let mut global_owner_update_tracker = Calculator::default();
    let mut local_owner_update_tracker = Calculator::default();
    let mut loc_clone_tracker = Calculator::default();
    let mut add_dependent_tracker = Calculator::default();
    let mut add_required_tracker = Calculator:: default();
    for _ in 0..args.num_blocks {
        println!("Creating {} transactions", args.block_size);
        let transactions: Vec<AnalyzedTransaction> = (0..args.block_size)
            .map(|_| {
                // randomly select a sender and receiver from accounts
                let mut rng = OsRng;

                let indices = rand::seq::index::sample(&mut rng, num_accounts, 2);
                let receiver = accounts[indices.index(1)].lock().unwrap();
                let mut sender = accounts[indices.index(0)].lock().unwrap();
                create_signed_p2p_transaction(&mut sender, vec![&receiver]).remove(0).into()
            })
            .collect();
        println!("Starting to partition");
        let now = Instant::now();
        let result = BlockPartitioner::partition(&partitioner, transactions, args.num_shards);
        let elapsed = now.elapsed();
        let global_owner_update = global_owner_update_tracker.new_val(SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["global_owner_update"]).get_sample_sum());
        let local_owner_update = local_owner_update_tracker.new_val(SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["local_owner_update"]).get_sample_sum());
        let loc_clone = loc_clone_tracker.new_val(SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["loc_clone"]).get_sample_sum());
        let add_dependent = add_dependent_tracker.new_val(SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["add_dependent"]).get_sample_sum());
        let add_required = add_required_tracker.new_val(SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["add_required"]).get_sample_sum());
        println!("global_owner_update={global_owner_update}");
        println!("local_owner_update={local_owner_update}");
        println!("loc_clone={loc_clone}");
        println!("add_dependent={add_dependent}");
        println!("add_required={add_required}");

        println!("Time taken to partition: {:?}", elapsed);
        // report_sub_block_matrix(&result);
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}

struct Calculator {
    val: f64,
}

impl Calculator {
    pub fn new_val(&mut self, val: f64) -> f64 {
        let ret = val - self.val;
        self.val = val;
        ret
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self {
            val: 0.0,
        }
    }
}
