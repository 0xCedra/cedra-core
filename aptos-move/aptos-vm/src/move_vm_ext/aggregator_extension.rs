// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::delta_ext::{addition, deserialize, serialize, subtraction, DeltaOp};
use aptos_crypto::hash::DefaultHasher;
use aptos_types::vm_status::StatusCode;
use better_any::{Tid, TidAble};
use move_deps::{
    move_binary_format::errors::{PartialVMError, PartialVMResult},
    move_core_types::{account_address::AccountAddress, gas_schedule::GasCost},
    move_table_extension::{TableHandle, TableResolver},
    move_vm_runtime::{
        native_functions,
        native_functions::{NativeContext, NativeFunctionTable},
    },
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        pop_arg,
        values::{Reference, Struct, StructRef, Value},
    },
};
use smallvec::smallvec;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    convert::TryInto,
};

/// Describes the state of each aggregator instance.
#[derive(Clone, Copy, PartialEq)]
pub enum AggregatorState {
    // If aggregator stores a known value.
    Data,
    // If aggregator stores a non-negative delta.
    PositiveDelta,
}

/// Uniquely identifies each aggregator instance in storage.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregatorID {
    // A handle that is shared accross all aggregator instances created by the
    // same `AggregatorFactory` and which is used for fine-grained storage
    // access.
    pub handle: u128,
    // Unique key associated with each aggregator instance. Generated by
    // taking the hash of transaction which creates an aggregator and the
    // number of aggregators that were created by this transaction so far.
    pub key: u128,
}

impl AggregatorID {
    fn new(handle: u128, key: u128) -> Self {
        AggregatorID { handle, key }
    }
}

/// Internal aggregator data structure.
struct Aggregator {
    // Describes a value of an aggregator.
    value: u128,
    // Describes a state of an aggregator.
    state: AggregatorState,
    // Describes an upper bound of an aggregator. If `value` exceeds it, the
    // aggregator overflows.
    // TODO: Currently this is a single u128 value since we use 0 as a trivial
    // lower bound. If we want to support custom lower bounds, or have more
    // complex postconditions, we should factor this out in its own struct.
    limit: u128,
}

impl Aggregator {
    /// Implements logic for adding to an aggregator.
    fn add(&mut self, value: u128) -> PartialVMResult<()> {
        // At this point, aggregator holds a positive delta or knows the value.
        // Hence, we can add, of course checking for overflow.
        self.value = addition(self.value, value, self.limit)?;
        Ok(())
    }

    /// Implements logic for subtracting from an aggregator.
    fn sub(&mut self, value: u128) -> PartialVMResult<()> {
        match self.state {
            AggregatorState::Data => {
                // Aggregator knows the value, therefore we can subtract
                // checking we don't drop below zero.
                self.value = subtraction(self.value, value)?;
                Ok(())
            }
            // For now, `aggregator::sub` always materializes the value, so
            // this should be unreachable.
            // TODO: support non-materialized subtractions.
            AggregatorState::PositiveDelta => {
                unreachable!("subtraction always materializes the value")
            }
        }
    }

    /// Implements logic for materializing the value of an aggregator. As a
    /// result, the aggregator knows it value (i.e. its state changed to
    /// `Data`).
    fn materialize(
        &mut self,
        context: &NativeAggregatorContext,
        id: &AggregatorID,
    ) -> PartialVMResult<()> {
        // If aggregator has already been materialized, return immediately.
        if self.state == AggregatorState::Data {
            return Ok(());
        }

        // Otherwise, we have a delta and have to go to storage and apply it.
        // In theory, any delta will be applied to existing value. However,
        // something may go wrong, so we guard by throwing an error in
        // extension.
        let key_bytes = serialize(&id.key);
        context
            .resolver
            .resolve_table_entry(&TableHandle(id.handle), &key_bytes)
            .map_err(|_| extension_error("could not find the value of the aggregator"))?
            .map_or(
                Err(extension_error(
                    "could not find the value of the aggregator",
                )),
                |bytes| {
                    // The only remaining case is PositiveDelta. Assert just in
                    // case.
                    debug_assert!(self.state == AggregatorState::PositiveDelta);

                    // Get the value from the storage and try to apply the delta
                    // to it. If application succeeds, we change the state of the
                    // aggregator. Otherwise the error is propagated to the caller.
                    let base = deserialize(&bytes);
                    self.value = addition(base, self.value, self.limit)?;
                    self.state = AggregatorState::Data;
                    Ok(())
                },
            )
    }
}

/// Stores all information about aggregators (how many have been created or
/// removed), what are their states, etc. per context (i.e. single
/// transaction).
#[derive(Default)]
struct AggregatorData {
    // All aggregators that were created in the current context, stored as ids.
    new_aggregators: BTreeSet<AggregatorID>,
    // All aggregators that were destroyed in the current context, stored as ids.
    destroyed_aggregators: BTreeSet<AggregatorID>,
    // All aggregator instances that exist in the current context.
    aggregators: BTreeMap<AggregatorID, Aggregator>,
}

impl AggregatorData {
    /// Returns a mutable reference to an aggregator with `id` and a `limit`.
    /// If it does not exist in the current context (i.e. transaction that is
    /// currently executing did not initilize it), a new aggregator instance is
    /// created, with a zero-initialized value and in a delta state.
    /// Note: when we say "aggregator instance" here we refer to Rust struct and
    /// not to the Move aggregator.
    fn get_aggregator(&mut self, id: AggregatorID, limit: u128) -> &mut Aggregator {
        self.aggregators.entry(id).or_insert_with(|| Aggregator {
            value: 0,
            state: AggregatorState::PositiveDelta,
            limit,
        });
        self.aggregators.get_mut(&id).unwrap()
    }

    /// Returns the number of aggregators that are used in the current context.
    fn num_aggregators(&self) -> u128 {
        self.aggregators.len() as u128
    }

    /// Creates and a new Aggregator with a given `id` and a `limit`. The value
    /// of a new aggregator is always known, therefore it is created in a data
    /// state, with a zero-initialized value.
    fn create_new_aggregator(&mut self, id: AggregatorID, limit: u128) {
        let aggregator = Aggregator {
            value: 0,
            state: AggregatorState::Data,
            limit,
        };
        self.aggregators.insert(id, aggregator);
        self.new_aggregators.insert(id);
    }

    /// If aggregator has been used in this context, it is removed. Otherwise,
    /// it is marked for deletion.
    fn remove_aggregator(&mut self, id: AggregatorID) {
        // Aggregator no longer in use during this context: remove it.
        self.aggregators.remove(&id);

        if self.new_aggregators.contains(&id) {
            // Aggregator has been created in the same context. Therefore, no
            // side-effects.
            self.new_aggregators.remove(&id);
        } else {
            // Otherwise, aggregator has been created somewhere else.
            self.destroyed_aggregators.insert(id);
        }
    }
}

/// Represents a single aggregator change.
#[derive(Debug)]
pub enum AggregatorChange {
    // A value should be written to storage.
    Write(u128),
    // A delta should be applied to storage.
    Apply(DeltaOp),
    // A value should be deleted from the storage.
    Delete,
}

/// Represents changes made by all aggregators during this context. This change
/// set can be converted into appropriate `WriteSet` and `DeltaChangeSet` by the
/// user, e.g. VM session.
pub struct AggregatorChangeSet {
    pub changes: BTreeMap<AggregatorID, AggregatorChange>,
}

/// Native context that can be attached to VM `NativeContextExtensions`.
///
/// Note: table resolver is reused for fine-grained storage access.
#[derive(Tid)]
pub struct NativeAggregatorContext<'a> {
    txn_hash: u128,
    resolver: &'a dyn TableResolver,
    aggregator_data: RefCell<AggregatorData>,
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(txn_hash: u128, resolver: &'a dyn TableResolver) -> Self {
        Self {
            txn_hash,
            resolver,
            aggregator_data: Default::default(),
        }
    }

    /// Returns all changes made within this context (i.e. by a single
    /// transaction).
    pub fn into_change_set(self) -> AggregatorChangeSet {
        let NativeAggregatorContext {
            aggregator_data, ..
        } = self;
        let AggregatorData {
            destroyed_aggregators,
            aggregators,
            ..
        } = aggregator_data.into_inner();

        let mut changes = BTreeMap::new();

        // First, process all writes and deltas.
        for (id, aggregator) in aggregators {
            let Aggregator {
                value,
                state,
                limit,
            } = aggregator;

            let change = match state {
                AggregatorState::Data => AggregatorChange::Write(value),
                AggregatorState::PositiveDelta => {
                    let delta_op = DeltaOp::Addition { value, limit };
                    AggregatorChange::Apply(delta_op)
                }
            };
            changes.insert(id, change);
        }

        // Additionally, do not forget to delete destroyed values from storage.
        for id in destroyed_aggregators {
            changes.insert(id, AggregatorChange::Delete);
        }

        AggregatorChangeSet { changes }
    }
}

// ================================= Natives =================================

/// All aggregator native functions. For more details, refer to code in
/// `aggregator_factory.move` and `aggregator.move`.
pub fn aggregator_natives(aggregator_addr: AccountAddress) -> NativeFunctionTable {
    native_functions::make_table(
        aggregator_addr,
        &[
            ("aggregator", "add", native_add),
            ("aggregator", "read", native_read),
            ("aggregator", "remove_aggregator", native_remove_aggregator),
            ("aggregator", "sub", native_sub),
            (
                "aggregator_factory",
                "new_aggregator",
                native_new_aggregator,
            ),
        ],
    )
}

/// Move signature:
/// fun new_aggregator(
///   aggregator_factory: &mut AggregatorFactory,
///   limit: u128
/// ): Aggregator;
fn native_new_aggregator(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Extract fields: `limit` of the new aggregator and a `phantom_handle` of
    // the parent factory.
    let limit = pop_arg!(args, u128);
    let handle = get_handle(&pop_arg!(args, StructRef))?;

    // Get the current aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    // Every aggregator instance uses a unique key in its id. Here we can reuse
    // the strategy from `table` implementation: taking hash of transaction and
    // number of aggregator instances created so far and truncating them to
    // 128 bits.
    let txn_hash_buffer = u128::to_be_bytes(aggregator_context.txn_hash);
    let num_aggregators_buffer = u128::to_be_bytes(aggregator_data.num_aggregators());

    let mut hasher = DefaultHasher::new(&[0_u8; 0]);
    hasher.update(&txn_hash_buffer);
    hasher.update(&num_aggregators_buffer);
    let hash = hasher.finish();

    // TODO: Using u128 is not enough, and it should be u256 instead. For now,
    // just take first 16 bytes of the hash.
    let bytes = &hash.to_vec()[..16];
    let key = u128::from_be_bytes(bytes.try_into().expect("not enough bytes"));

    let id = AggregatorID::new(handle, key);
    aggregator_data.create_new_aggregator(id, limit);

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();

    // Return `Aggregator` Move struct to the user.
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::struct_(Struct::pack(vec![
            Value::u128(handle),
            Value::u128(key),
            Value::u128(limit),
        ]))],
    ))
}

/// Move signature:
/// fun add(aggregator: &mut Aggregator, value: u128);
fn native_add(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Get aggregator fields and a value to add.
    let value = pop_arg!(args, u128);
    let aggregator_ref = pop_arg!(args, StructRef);
    let (handle, key, limit) = get_aggregator_fields(&aggregator_ref)?;
    let id = AggregatorID::new(handle, key);

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, limit);

    aggregator.add(value)?;

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();
    Ok(NativeResult::ok(cost, smallvec![]))
}

/// Move signature:
/// fun read(aggregator: &Aggregator): u128;
fn native_read(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 1);
    let aggregator_ref = pop_arg!(args, StructRef);

    // Extract fields from aggregator struct reference.
    let (handle, key, limit) = get_aggregator_fields(&aggregator_ref)?;
    let id = AggregatorID::new(handle, key);

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, limit);

    // Materialize the value.
    aggregator.materialize(aggregator_context, &id)?;

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::u128(aggregator.value)],
    ))
}

/// Move signature:
/// fun sub(aggregator: &mut Aggregator, value: u128);
fn native_sub(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Get aggregator fields and a value to subtract.
    let value = pop_arg!(args, u128);
    let aggregator_ref = pop_arg!(args, StructRef);
    let (handle, key, limit) = get_aggregator_fields(&aggregator_ref)?;
    let id = AggregatorID::new(handle, key);

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, limit);

    // For first version of `Aggregator` (V1), subtraction always materializes
    // the value first. While this limits commutativity, it is sufficient for
    // now.
    // TODO: change this when we implement commutative subtraction.
    aggregator.materialize(aggregator_context, &id)?;
    aggregator.sub(value)?;

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();
    Ok(NativeResult::ok(cost, smallvec![]))
}

/// Move signature:
/// fun remove_aggregator(table_handle: u128, key: u128);
fn native_remove_aggregator(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Get aggregator id for removal.
    let key = pop_arg!(args, u128);
    let handle = pop_arg!(args, u128);
    let id = AggregatorID::new(handle, key);

    // Get aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    // Actually remove the aggregator.
    aggregator_data.remove_aggregator(id);

    // TODO: charge gas properly.
    let cost = GasCost::new(0, 0).total();
    Ok(NativeResult::ok(cost, smallvec![]))
}

// ================================ Utilities ================================

/// The index of the `phantom_table` field in the `AggregatorFactory` Move
/// struct.
const PHANTOM_TABLE_FIELD_INDEX: usize = 0;

/// The index of the `handle` field in the `Table` Move struct.
const TABLE_HANDLE_FIELD_INDEX: usize = 0;

/// Indices of `handle`, `key` and `limit` fields in the `Aggregator` Move
/// struct.
const HANDLE_FIELD_INDEX: usize = 0;
const KEY_FIELD_INDEX: usize = 1;
const LIMIT_FIELD_INDEX: usize = 2;

/// Given a reference to `AggregatorFactory` Move struct, returns the value of
/// `handle` field (from underlying `Table` struct).
fn get_handle(aggregator_table: &StructRef) -> PartialVMResult<u128> {
    aggregator_table
        .borrow_field(PHANTOM_TABLE_FIELD_INDEX)?
        .value_as::<StructRef>()?
        .borrow_field(TABLE_HANDLE_FIELD_INDEX)?
        .value_as::<Reference>()?
        .read_ref()?
        .value_as::<u128>()
}

/// Given a reference to `Aggregator` Move struct returns a field value at `index`.
fn get_aggregator_field(aggregator: &StructRef, index: usize) -> PartialVMResult<Value> {
    let field_ref = aggregator.borrow_field(index)?.value_as::<Reference>()?;
    field_ref.read_ref()
}

///  Given a reference to `Aggregator` Move struct, returns a tuple of its
///  fields: (`handle`, `key`, `limit`).
fn get_aggregator_fields(aggregator: &StructRef) -> PartialVMResult<(u128, u128, u128)> {
    let handle = get_aggregator_field(aggregator, HANDLE_FIELD_INDEX)?.value_as::<u128>()?;
    let key = get_aggregator_field(aggregator, KEY_FIELD_INDEX)?.value_as::<u128>()?;
    let limit = get_aggregator_field(aggregator, LIMIT_FIELD_INDEX)?.value_as::<u128>()?;
    Ok((handle, key, limit))
}

/// Returns partial VM error on extension failure.
fn extension_error(message: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(message.to_string())
}

// ================================= Tests =================================

#[cfg(test)]
mod test {
    use super::*;
    use claim::assert_matches;
    use move_deps::move_vm_test_utils::BlankStorage;
    use once_cell::sync::Lazy;

    static DUMMY_RESOLVER: Lazy<BlankStorage> = Lazy::new(|| BlankStorage);

    fn test_id(key: u128) -> AggregatorID {
        AggregatorID::new(0, key)
    }

    fn test_set_up(context: &NativeAggregatorContext) {
        let mut aggregator_data = context.aggregator_data.borrow_mut();

        // Aggregators with data.
        aggregator_data.create_new_aggregator(test_id(0), 1000);
        aggregator_data.create_new_aggregator(test_id(1), 1000);
        aggregator_data.create_new_aggregator(test_id(2), 1000);

        // Aggregators with delta.
        aggregator_data.get_aggregator(test_id(3), 1000);
        aggregator_data.get_aggregator(test_id(4), 1000);
        aggregator_data.get_aggregator(test_id(5), 10);

        // Different cases of aggregator removal.
        aggregator_data.remove_aggregator(test_id(0));
        aggregator_data.remove_aggregator(test_id(3));
        aggregator_data.remove_aggregator(test_id(6));
    }

    #[test]
    fn test_into_change_set() {
        let context = NativeAggregatorContext::new(0, &*DUMMY_RESOLVER);
        test_set_up(&context);

        let AggregatorChangeSet { changes } = context.into_change_set();

        assert!(!changes.contains_key(&test_id(0)));

        assert_matches!(
            changes.get(&test_id(1)).unwrap(),
            AggregatorChange::Write(0)
        );
        assert_matches!(
            changes.get(&test_id(2)).unwrap(),
            AggregatorChange::Write(0)
        );

        assert_matches!(changes.get(&test_id(3)).unwrap(), AggregatorChange::Delete);

        assert_matches!(
            changes.get(&test_id(4)).unwrap(),
            AggregatorChange::Apply(DeltaOp::Addition {
                value: 0,
                limit: 1000
            })
        );
        assert_matches!(
            changes.get(&test_id(5)).unwrap(),
            AggregatorChange::Apply(DeltaOp::Addition {
                value: 0,
                limit: 10
            })
        );

        assert_matches!(changes.get(&test_id(6)).unwrap(), AggregatorChange::Delete);
    }
}
