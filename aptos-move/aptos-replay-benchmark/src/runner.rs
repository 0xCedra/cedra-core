// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use std::time::Instant;

/// Holds configuration for running the benchmarks and measuring the time taken.
pub struct BenchmarkRunner {
    concurrency_levels: Vec<usize>,
    num_repeats: usize,
    measure_block_time: bool,
}

impl BenchmarkRunner {
    pub fn new(
        concurrency_levels: Vec<usize>,
        num_repeats: usize,
        measure_block_time: bool,
    ) -> Self {
        Self {
            concurrency_levels,
            num_repeats,
            measure_block_time,
        }
    }

    // TODO:
    //   This measures execution time from a cold-start. Ideally, we want to warm-up with executing
    //   1-2 blocks prior to selected range, but not timing them.
    pub fn measure_execution_time(&self, blocks: &[Block]) {
        for concurrency_level in &self.concurrency_levels {
            if self.measure_block_time {
                self.measure_block_execution_time(blocks, *concurrency_level);
            } else {
                self.measure_overall_execution_time(blocks, *concurrency_level);
            }
        }
    }

    /// Runs a sequence of blocks, measuring execution time for each block. The median is reported.
    fn measure_block_execution_time(&self, blocks: &[Block], concurrency_level: usize) {
        let mut times = Vec::with_capacity(blocks.len());
        for _ in blocks {
            times.push(Vec::with_capacity(self.num_repeats));
        }

        for i in 0..self.num_repeats {
            let executor = AptosVMBlockExecutor::new();
            for (idx, block) in blocks.iter().enumerate() {
                let start_time = Instant::now();
                block.run(&executor, concurrency_level);
                let time = start_time.elapsed().as_millis();
                println!(
                    "[{}/{}] Block {} execution time is {}ms",
                    i + 1,
                    self.num_repeats,
                    idx + 1,
                    time,
                );
                times[idx].push(time);
            }
        }

        for (idx, mut time) in times.into_iter().enumerate() {
            time.sort();
            println!(
                "Block {} median execution time is {}ms",
                idx + 1,
                time[self.num_repeats / 2],
            );
        }
    }

    /// Runs the sequence of blocks, measuring end-to-end execution time.
    fn measure_overall_execution_time(&self, blocks: &[Block], concurrency_level: usize) {
        let mut times = Vec::with_capacity(self.num_repeats);
        for i in 0..self.num_repeats {
            let start_time = Instant::now();
            let executor = AptosVMBlockExecutor::new();
            for block in blocks {
                block.run(&executor, concurrency_level);
            }
            let time = start_time.elapsed().as_millis();
            println!(
                "[{}/{}] Overall execution time is {}ms",
                i + 1,
                self.num_repeats,
                time,
            );
            times.push(time);
        }
        times.sort();
        println!(
            "Overall median execution time is {}ms\n",
            times[self.num_repeats / 2],
        );
    }
}
