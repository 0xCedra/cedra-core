
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::{CircuitInputSignals, Padded}, config::CircuitPaddingConfig,
};
use std::iter::zip;
use rand::{distributions::Alphanumeric, Rng}; // 0.8
use ark_bn254::Fr;




fn generate_string_bodies_input() -> String {
    let mut rng = rand::thread_rng();

    let len = 13;

    let mut s : Vec<u8> = rng
        .sample_iter(&Alphanumeric)
        .take(len)
        .collect::<String>()
        .as_bytes()
        .into();

     let num_to_replace = rng.gen_range(0,len);
     let to_replace : Vec<usize> = (0..num_to_replace).map(|_| rng.gen_range(0, len)).collect();
     let replace_with_escaped_quote : Vec<bool> = (0..num_to_replace).map(|_| rng.gen_bool(0.5)).collect();

     for (i,should_replace_with_escaped_quote) in zip(to_replace, replace_with_escaped_quote) {
         if should_replace_with_escaped_quote && i > 0 {
             s[i-1] = b'\\';
             s[i] = b'"';
         } else {
             s[i] = b'"';
         }
     }

     String::from_utf8_lossy(&s).into_owned()
}

fn format_quotes_array(q: &[bool]) -> String {
    q.iter()
     .map(|b| match b { true => "1", false => "0" })
     .collect::<Vec<&str>>()
     .concat()
}

pub fn calc_string_bodies(s: &str) -> Vec<bool> {
    let bytes = s.as_bytes();
    let mut string_bodies = vec![false; s.len()];
    let mut quotes = vec![false; s.len()];
    let mut quote_parity = vec![false; s.len()];

    quotes[0] = bytes[0] == b'"';
    quote_parity[0] = bytes[0] == b'"';
    for i in 1..bytes.len() {
        let mut prev_is_odd_backslash = false;
        for j in (0..i).rev() {
            if bytes[j] != b'\\' { break; }
            println!("{}: {}", j, bytes[j]);
            prev_is_odd_backslash = !prev_is_odd_backslash;
        }
        quotes[i] = bytes[i] == b'"' && !prev_is_odd_backslash;
        quote_parity[i] = if  quotes[i] { !quote_parity[i-1] } else { quote_parity[i-1] };
    }

    string_bodies[0] = false;
    for i in 1..bytes.len() {
        string_bodies[i] = quote_parity[i] && quote_parity[i-1];
    }

    println!("string       : {}", s);
    println!("quote_parity : {}", format_quotes_array(&quote_parity));
    println!("string_bodies: {}", format_quotes_array(&string_bodies));

    string_bodies
}

#[test]
fn is_whitespace_test() {
    let circuit_handle = TestCircuitHandle::new("misc/is_whitespace_test.circom").unwrap();


    for c in 0u8..=127u8 {
        let config = CircuitPaddingConfig::new();

        let circuit_input_signals = CircuitInputSignals::new()
            .byte_input("char", c)
            .bool_input("result", (c as char).is_whitespace())
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{}: {:?}", c, result);
        assert!(result.is_ok());
    }
}



#[test]
fn string_bodies_test() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();

    let s = "\"123\" 456 \"7\"";
    let quotes = &[0u8, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0];
    let quotes_b : Vec<bool> = 
        quotes
        .iter()
        .map(|b| b == &1u8)
        .collect();

    assert_eq!(quotes_b, calc_string_bodies(s));



    let config = CircuitPaddingConfig::new()
        .max_length("in", 13)
        .max_length("out", 13);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("in", s)
        .bytes_input("out", quotes)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_ok());
}


#[test]
fn string_bodies_test_2() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();

    let s = "\"12\\\"456\" \"7\"";
    let quotes = &[0u8, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0];
    let quotes_b : Vec<bool> = 
        quotes
        .iter()
        .map(|b| b == &1u8)
        .collect();

    assert_eq!(quotes_b, calc_string_bodies(s));


    let config = CircuitPaddingConfig::new()
        .max_length("in", 13)
        .max_length("out", 13);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("in", s)
        .bytes_input("out", quotes)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_ok());
}

#[test]
fn string_bodies_test_random() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();


    for iter in 0..128 { 
        let s = generate_string_bodies_input();
        let quotes = calc_string_bodies(&s);

        let config = CircuitPaddingConfig::new()
            .max_length("in", 13)
            .max_length("out", 13);

        let circuit_input_signals = CircuitInputSignals::new()
            .str_input("in", &s)
            .bools_input("out", &quotes)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn string_bodies_test_prefix_quotes() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();


    for i in 0..13 { 
        let mut bytes = vec![b'a'; 13];
        for j in 0..i {
            bytes[j] = b'"';
        }
        let s = String::from_utf8_lossy(&bytes);

        let quotes = calc_string_bodies(&s);

        let config = CircuitPaddingConfig::new()
            .max_length("in", 13)
            .max_length("out", 13);

        let circuit_input_signals = CircuitInputSignals::new()
            .str_input("in", &s)
            .bools_input("out", &quotes)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}


#[test]
fn string_bodies_test_zjma() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();

    let s = "\"abc\\\\\"";
    let quotes = calc_string_bodies(&s);

    let config = CircuitPaddingConfig::new()
        .max_length("in", 13)
        .max_length("out", 13);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("in", s)
        .bools_input("out", &quotes)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_ok());
}

#[test]
fn calculate_total_test() {
    let circuit_handle = TestCircuitHandle::new("misc/calculate_total_test.circom").unwrap();

    let mut rng = rand::thread_rng();

    for i in 0..256 {
        let nums : Vec<Fr> = (0..10)
            .map(|_| Fr::from(rng.gen::<u64>()) )
            .collect();

        let sum : Fr = nums.iter().sum();


        let config = CircuitPaddingConfig::new()
            .max_length("nums", 10);

        let circuit_input_signals = CircuitInputSignals::new()
            .frs_input("nums", &nums)
            .fr_input("sum", sum)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}


#[test]
fn assert_equal_if_true_test() {
    let circuit_handle = TestCircuitHandle::new("misc/assert_equal_if_true_test.circom").unwrap();

    let mut rng = rand::thread_rng();

    for i in 0..256 {

        let (nums, are_equal) = 
            if rng.gen_bool(0.5) {
                let nums : Vec<Fr> = (0..2)
                    .map(|_| Fr::from(rng.gen::<u64>()) )
                    .collect();

                let are_equal = nums[0] == nums[1];
                (nums, are_equal)
            } else {
                let num = Fr::from(rng.gen::<u64>());
                let nums : Vec<Fr> = vec![num, num];

                (nums, true)
            };




        let config = CircuitPaddingConfig::new()
            .max_length("in", 2);

        let circuit_input_signals = CircuitInputSignals::new()
            .frs_input("in", &nums)
            .bool_input("bool", are_equal)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}
