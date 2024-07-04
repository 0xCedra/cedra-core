// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Simple CLI tool that checks if a Move transactional test works as expected

use arbitrary::Unstructured;
use clap::Parser;
use move_smith::{
    utils::{compile_move_code, run_transactional_test, TransactionalResult},
    CodeGenerator, MoveSmith,
};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// The input file to check
    #[clap(short('f'), long)]
    input_file: PathBuf,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let bytes = fs::read(&args.input_file).unwrap();
    let mut u = Unstructured::new(&bytes);
    let mut smith = MoveSmith::default();
    match smith.generate(&mut u) {
        Ok(()) => println!("Parsed raw input successfully"),
        Err(e) => {
            println!("Failed to parse raw input: {:?}", e);
            std::process::exit(1);
        },
    };
    let code = smith.get_compile_unit().emit_code();
    println!("Loaded code from file: {:?}", args.input_file);

    compile_move_code(code.clone(), true, false);
    println!("Compiled code with V1 did not panic");

    compile_move_code(code.clone(), false, true);
    println!("Compiled code with V2 did not panic");

    match run_transactional_test(code, &smith.config.take()) {
        TransactionalResult::Ok => println!("Running as transactional test passed"),
        TransactionalResult::WarningsOnly => {
            println!("Running as transactional test passed with all warnings")
        },
        TransactionalResult::IgnoredErr(_) => {
            println!("Running as transactional test passed with errors ignored")
        },
        TransactionalResult::Err(msg) => {
            println!("Transactional test failed: {}", msg);
            std::process::exit(1);
        },
    }
}
