module 0x1::aggregator_v2_test {
    use std::signer;

    use aptos_framework::aggregator_v2::{Self, Aggregator, AggregatorSnapshot};
    use aptos_std::table::{Self, Table};

    /// When checking the value of aggregator fails.
    const ENOT_EQUAL: u64 = 17;

    /// Resource to store aggregators. Each aggregator is associated with a
    /// determinictic integer value, for testing purposes.
    struct AggregatorStore has key, store {
        aggregators: Table<u64, Aggregator>,
        aggregator_snapshots_u128: Table<u64, AggregatorSnapshot<u128>>,
        aggregator_snapshots_u64: Table<u64, AggregatorSnapshot<u64>>,
    }

    /// Initializes a fake resource which holds aggregators.
    public entry fun initialize(account: &signer) {
        let aggregators = table::new();
        let aggregator_snapshots_u128 = table::new();
        let aggregator_snapshots_u64 = table::new();
        let store = AggregatorStore { aggregators, aggregator_snapshots_u128, aggregator_snapshots_u64 };
        move_to(account, store);
    }

    /// Checks that the ith aggregator has expected value. Useful to inject into
    /// transaction block to verify successful and correct execution.
    public entry fun check(account: &signer, i: u64, expected: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        let actual = aggregator_v2::read(aggregator);
        assert!(actual == expected, ENOT_EQUAL)
    }

    //
    // Testing scripts.
    //

    public entry fun new(account: &signer, i: u64, limit: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = aggregator_v2::create_aggregator(limit);
        table::add(aggregators, i, aggregator);
    }

    public entry fun try_add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_add(aggregator, value);
    }

    public entry fun try_sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, value);
    }

    public entry fun try_sub_add(account: &signer, i: u64, a: u128, b: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, a);
        aggregator_v2::try_add(aggregator, b);
    }

    public entry fun materialize(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        aggregator_v2::read(aggregator);
    }

    public entry fun materialize_and_try_add(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::read(aggregator);
        aggregator_v2::try_add(aggregator, value);
    }

    public entry fun materialize_and_try_sub(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::read(aggregator);
        aggregator_v2::try_sub(aggregator, value);
    }

    public entry fun try_add_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_add(aggregator, value);
        aggregator_v2::read(aggregator);
    }

    public entry fun try_sub_and_materialize(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        aggregator_v2::try_sub(aggregator, value);
        aggregator_v2::read(aggregator);
    }

    public entry fun snapshot(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        let aggregator_snapshots_u128 = &mut borrow_global_mut<AggregatorStore>(addr).aggregator_snapshots_u128;
        let aggregator_snapshot = aggregator_v2::snapshot(aggregator);
        table::add(aggregator_snapshots_u128, i, aggregator_snapshot);
    }

    public entry fun snapshot_with_u64_limit(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &borrow_global<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow(aggregators, i);
        let aggregator_snapshots_u64 = &mut borrow_global_mut<AggregatorStore>(addr).aggregator_snapshots_u64;
        let aggregator_snapshot = aggregator_v2::snapshot_with_u64_limit(aggregator);
        table::add(aggregator_snapshots_u64, i, aggregator_snapshot);
    }

    public entry fun read_snapshot(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregator_snapshots_u128 = &borrow_global<AggregatorStore>(addr).aggregator_snapshots_u128;
        let aggregator_snapshot = table::borrow(aggregator_snapshots_u128, i);
        aggregator_v2::read_snapshot(aggregator_snapshot);
    }

    public entry fun read_snapshot_with_u64_limit(account: &signer, i: u64) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregator_snapshots_u64 = &borrow_global<AggregatorStore>(addr).aggregator_snapshots_u64;
        let aggregator_snapshot = table::borrow(aggregator_snapshots_u64, i);
        aggregator_v2::read_snapshot(aggregator_snapshot);
    }

    public entry fun try_add_snapshot(account: &signer, i: u64, value: u128) acquires AggregatorStore {
        let addr = signer::address_of(account);
        let aggregators = &mut borrow_global_mut<AggregatorStore>(addr).aggregators;
        let aggregator = table::borrow_mut(aggregators, i);
        let aggregator_snapshot_1 = aggregator_v2::snapshot(aggregator);
        aggregator_v2::try_add(aggregator, value);
        let aggregator_snapshot_2 = aggregator_v2::snapshot(aggregator);
        aggregator_v2::try_add(aggregator, value);
        let aggregator_snapshot_3 = aggregator_v2::snapshot(aggregator);
        let snapshot_value_1 = aggregator_v2::read_snapshot<u128>(&aggregator_snapshot_1);
        let snapshot_value_2 = aggregator_v2::read_snapshot<u128>(&aggregator_snapshot_2);
        let snapshot_value_3 = aggregator_v2::read_snapshot<u128>(&aggregator_snapshot_3);
        assert!(snapshot_value_2 == snapshot_value_1 + value, ENOT_EQUAL);
        assert!(snapshot_value_3 == snapshot_value_2 + value, ENOT_EQUAL);
    }
}
