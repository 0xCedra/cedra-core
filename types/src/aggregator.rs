// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::values::{Struct, Value};
use once_cell::sync::Lazy;
use std::str::FromStr;

/// Ephemeral identifier type used by delayed fields (aggregators, snapshots)
/// during execution.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DelayedFieldID(u64);

impl DelayedFieldID {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn new_with_width(unique_index: u32, width: usize) -> Self {
        assert!(width < 1usize << 10, "Delayed field width must be <= 10");
        Self(((unique_index as u64) << 10) | width as u64)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_utf8_fixed_size(&self) -> Result<Vec<u8>, PanicError> {
        let width = self.extract_width();
        let approx_width = size_u32_as_uleb128(width);
        if width <= approx_width + 2 {
            return Err(code_invariant_error(format!("aggregators_v2::DerivedString size issue for id {self:?}: width: {width}, approx_width: {approx_width}")));
        }
        Ok(u64_to_fixed_size_utf8_bytes(
            self.as_u64(),
            width - approx_width - 2,
        ))
    }

    pub fn extract_width(&self) -> usize {
        (self.0 & ((1 << 10) - 1)) as usize
    }

    pub fn into_derived_string_struct(self) -> Result<Value, PanicError> {
        bytes_and_width_to_derived_string_struct(self.as_utf8_fixed_size()?, self.extract_width())
    }
}

// Used for ID generation from u32/u64 counters.
impl From<u64> for DelayedFieldID {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

// Used for ID generation from u32/u64 counters with width.
impl From<(u32, usize)> for DelayedFieldID {
    fn from(value: (u32, usize)) -> Self {
        let (index, width) = value;
        Self::new_with_width(index, width)
    }
}

// Represents something that should never happen - i.e. a code invariant error,
// which we would generally just panic, but since we are inside of the VM,
// we cannot do that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanicError {
    CodeInvariantError(String),
}

impl ToString for PanicError {
    fn to_string(&self) -> String {
        match self {
            PanicError::CodeInvariantError(e) => e.clone(),
        }
    }
}

impl From<PanicError> for PartialVMError {
    fn from(err: PanicError) -> Self {
        match err {
            PanicError::CodeInvariantError(msg) => {
                PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR)
                    .with_message(msg)
            },
        }
    }
}

pub trait ExtractUniqueIndex: Sized {
    fn extract_unique_index(&self) -> u32;
}

/// Types which implement this trait can be converted to a Move value.
pub trait TryIntoMoveValue: Sized {
    type Error: std::fmt::Debug;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error>;
}

/// Types which implement this trait can be constructed from a Move value.
pub trait TryFromMoveValue: Sized {
    // Allows to pass extra information from the caller.
    type Hint;
    type Error: std::fmt::Debug;

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        hint: &Self::Hint,
    ) -> Result<(Self, usize), Self::Error>;
}

impl ExtractUniqueIndex for DelayedFieldID {
    fn extract_unique_index(&self) -> u32 {
        (self.0 >> 10).try_into().unwrap()
    }
}

impl TryIntoMoveValue for DelayedFieldID {
    type Error = PanicError;

    fn try_into_move_value(self, layout: &MoveTypeLayout) -> Result<Value, Self::Error> {
        Ok(match layout {
            MoveTypeLayout::U64 => Value::u64(self.as_u64()),
            MoveTypeLayout::U128 => Value::u128(self.as_u64() as u128),
            layout if is_derived_string_struct_layout(layout) => {
                // Here, we make sure we convert identifiers to fixed-size Move
                // values. This is needed because we charge gas based on the resource
                // size with identifiers inside, and so it has to be deterministic.

                self.into_derived_string_struct()?
            },
            _ => {
                return Err(code_invariant_error(format!(
                    "Failed to convert {:?} into a Move value with {} layout",
                    self, layout
                )))
            },
        })
    }
}

impl TryFromMoveValue for DelayedFieldID {
    type Error = PanicError;
    type Hint = ();

    fn try_from_move_value(
        layout: &MoveTypeLayout,
        value: Value,
        _hint: &Self::Hint,
    ) -> Result<(Self, usize), Self::Error> {
        // Since we put the value there, we should be able to read it back,
        // unless there is a bug in the code - so we expect_ok() throughout.
        let (id, width) = match layout {
            MoveTypeLayout::U64 => (Self::new(expect_ok(value.value_as::<u64>())?), 8),
            MoveTypeLayout::U128 => (
                Self::new(expect_ok(value.value_as::<u128>()).and_then(u128_to_u64)?),
                16,
            ),
            layout if is_derived_string_struct_layout(layout) => {
                let (bytes, width) = value
                    .value_as::<Struct>()
                    .and_then(derived_string_struct_to_bytes_and_length)
                    .map_err(|e| {
                        code_invariant_error(format!(
                            "couldn't extract derived string struct: {:?}",
                            e
                        ))
                    })?;
                let id = Self::new(from_utf8_bytes(bytes)?);
                (id, width)
            },
            // We use value to ID conversion in serialization.
            _ => {
                return Err(code_invariant_error(format!(
                    "Failed to convert a Move value with {} layout into an identifier",
                    layout
                )))
            },
        };
        if id.extract_width() != width {
            return Err(code_invariant_error(format!(
                "Extracted identifier has a wrong width: id={id:?}, width={width}, expected={}",
                id.extract_width(),
            )));
        }

        Ok((id, width))
    }
}

fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PanicError {
    let msg = format!(
        "Delayed logic code invariant broken (there is a bug in the code), {:?}",
        message
    );
    println!("ERROR: {}", msg);
    // cannot link aptos_logger in aptos-types crate
    // error!("{}", msg);
    PanicError::CodeInvariantError(msg)
}

fn expect_ok<V, E: std::fmt::Debug>(value: Result<V, E>) -> Result<V, PanicError> {
    value.map_err(code_invariant_error)
}

/// Returns true if the type layout corresponds to a String, which should be a
/// struct with a single byte vector field.
fn is_string_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout::*;
    if let Struct(move_struct) = layout {
        if let [Vector(elem)] = move_struct.fields().iter().as_slice() {
            if let U8 = elem.as_ref() {
                return true;
            }
        }
    }
    false
}

pub fn is_derived_string_struct_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout::*;
    if let Struct(move_struct) = layout {
        if let [value_field, Vector(padding_elem)] = move_struct.fields().iter().as_slice() {
            if is_string_layout(value_field) {
                if let U8 = padding_elem.as_ref() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn bytes_to_string(bytes: Vec<u8>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(bytes)]))
}

pub fn string_to_bytes(value: Struct) -> Result<Vec<u8>, PanicError> {
    expect_ok(value.unpack())?
        .collect::<Vec<Value>>()
        .pop()
        .map_or_else(
            || Err(code_invariant_error("Unable to extract bytes from String")),
            |v| expect_ok(v.value_as::<Vec<u8>>()),
        )
}

pub fn bytes_and_width_to_derived_string_struct(
    bytes: Vec<u8>,
    width: usize,
) -> Result<Value, PanicError> {
    let value_width = bcs_size_of_byte_array(bytes.len());
    if value_width + 1 > width {
        return Err(code_invariant_error(format!(
            "aggregators_v2::DerivedString size issue: value_width: {value_width}, width: {width}"
        )));
    }

    let padding_len = width - value_width - 1;
    if size_u32_as_uleb128(padding_len) > 1 {
        return Err(code_invariant_error(format!("aggregators_v2::DerivedString size issue: value_width: {value_width}, width: {width}, padding_len: {padding_len}")));
    }

    Ok(Value::struct_(Struct::pack(vec![
        bytes_to_string(bytes),
        Value::vector_u8(vec![0; padding_len]),
    ])))
}

pub fn u64_to_fixed_size_utf8_bytes(value: u64, width: usize) -> Vec<u8> {
    // Maximum u64 identifier size is 20 characters. We need a fixed size to
    // ensure identifiers have the same size all the time for all validators,
    // to ensure consistent and deterministic gas charging.
    format!("{:0>width$}", value, width = width)
        .to_string()
        .into_bytes()
}

pub static U64_MAX_DIGITS: Lazy<usize> = Lazy::new(|| u64::MAX.to_string().len());
pub static U128_MAX_DIGITS: Lazy<usize> = Lazy::new(|| u128::MAX.to_string().len());

pub fn to_utf8_bytes(value: impl ToString) -> Vec<u8> {
    value.to_string().into_bytes()
}

pub fn from_utf8_bytes<T: FromStr>(bytes: Vec<u8>) -> Result<T, PanicError> {
    String::from_utf8(bytes)
        .map_err(|e| code_invariant_error(format!("Unable to convert bytes to string: {}", e)))?
        .parse::<T>()
        .map_err(|_| code_invariant_error("Unable to parse string".to_string()))
}

pub fn derived_string_struct_to_bytes_and_length(
    value: Struct,
) -> PartialVMResult<(Vec<u8>, usize)> {
    let mut fields = value.unpack()?.collect::<Vec<Value>>();
    if fields.len() != 2 {
        return Err(
            PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(
                format!(
                    "aggregators_v2::DerivedString has wrong number of fields: {:?}",
                    fields.len()
                ),
            ),
        );
    }
    let padding = fields.pop().unwrap().value_as::<Vec<u8>>()?;
    let value = fields.pop().unwrap();
    let string_bytes = string_to_bytes(value.value_as::<Struct>()?)?;
    let string_len = string_bytes.len();
    Ok((
        string_bytes,
        bcs_size_of_byte_array(string_len) + bcs_size_of_byte_array(padding.len()),
    ))
}

pub fn u128_to_u64(value: u128) -> Result<u64, PanicError> {
    u64::try_from(value).map_err(|_| code_invariant_error("Cannot cast u128 into u64".to_string()))
}

pub fn size_u32_as_uleb128(mut value: usize) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        // 7 (lowest) bits of data get written in a single byte.
        len += 1;
        value >>= 7;
    }
    len
}

pub fn bcs_size_of_byte_array(length: usize) -> usize {
    size_u32_as_uleb128(length) + length
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_ok, assert_ok_eq};

    #[test]
    fn test_fixed_string_id_1() {
        let encoded = u64_to_fixed_size_utf8_bytes(7, 30);
        assert_eq!(encoded.len(), 30);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "000000000000000000000000000007");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, 7);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), 7);
    }

    #[test]
    fn test_fixed_string_id_2() {
        let encoded = u64_to_fixed_size_utf8_bytes(u64::MAX, 20);
        assert_eq!(encoded.len(), 20);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "18446744073709551615");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, u64::MAX);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), u64::MAX);
    }

    #[test]
    fn test_fixed_string_id_3() {
        let encoded = u64_to_fixed_size_utf8_bytes(0, 20);
        assert_eq!(encoded.len(), 20);

        let decoded_string = assert_ok!(String::from_utf8(encoded.clone()));
        assert_eq!(decoded_string, "00000000000000000000");

        let decoded = assert_ok!(decoded_string.parse::<u64>());
        assert_eq!(decoded, 0);
        assert_ok_eq!(from_utf8_bytes::<u64>(encoded), 0);
    }
}
