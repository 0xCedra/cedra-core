/// This module provides foundations to create aggregators in the system.
///
/// Design rationale (V1)
/// =====================
/// First, we encourage the reader to see rationale of `Aggregator` in
/// `aggregator.move`.
///
/// Recall that the value of any aggregator can be identified in storage by
/// (handle, key) pair. How this pair can be generated? Short answer: with
/// `AggregatorFactory`!
///
/// `AggregatorFactory` is a struct that can be stored as a resource on some
/// account and which contains a `phantom_table` field. When the factory is
/// initialized, we initialize this table. Importantly, table initialization
/// only generates a uniue table `handle` - something we can reuse.
///
/// When the user wants to create a new aggregator, he/she calls a constructor
/// provided by the factory (`new_aggregator(..)`). This constructor generates
/// a unique key, which with the handle is used to initialize `Aggregator` struct.
///
/// Use cases
/// =========
/// We limit the usage of `AggregatorFactory` by only storing it on the core
/// account.
///
/// When something whants to use an aggregator, the factory is queried and an
/// aggregator instance is created. Once aggregator is no longer in use, it
/// should be destroyed by the user.
module aptos_framework::aggregator_factory {
    use std::error;
    use std::signer;

    use aptos_framework::aggregator::{Self, Aggregator};
    use aptos_framework::system_addresses;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::timestamp;

    /// When aggregator factory has already been published.
    const EAGGREGATOR_FACTORY_EXISTS: u64 = 1;

    /// Struct that creates aggregators.
    struct AggregatorFactory has key {
        phantom_table: Table<u128, u128>,
    }

    /// Creates a new factory for aggregators.
    public fun initialize_aggregator_factory(account: &signer) {
        // Currently `AggregatorFactory` can live only on aptos framework and
        // should be created during genesis.
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(
            !exists<AggregatorFactory>(signer::address_of(account)),
            error::already_exists(EAGGREGATOR_FACTORY_EXISTS)
        );

        let aggregator_factory = AggregatorFactory {
            phantom_table: table::new()
        };
        move_to(account, aggregator_factory);
    }

    /// Creates a new aggregator instance associated with this `aggregator_factory`
    /// and which overflows on exceeding `limit`.
    public(friend) native fun new_aggregator(aggregator_factory: &mut AggregatorFactory, limit: u128): Aggregator;

    #[test(account = @aptos_framework)]
    fun test_can_add_and_sub_and_read(account: signer) acquires AggregatorFactory {
        initialize_aggregator_factory(&account);

        let addr = signer::address_of(&account);
        let aggregator_factory = borrow_global_mut<AggregatorFactory>(addr);

        let aggregator = new_aggregator(aggregator_factory, /*limit=*/1000);

        aggregator::add(&mut aggregator, 12);
        assert!(aggregator::read(&aggregator) == 12, 0);

        aggregator::add(&mut aggregator, 3);
        assert!(aggregator::read(&aggregator) == 15, 0);

        aggregator::add(&mut aggregator, 3);
        aggregator::add(&mut aggregator, 2);
        aggregator::sub(&mut aggregator, 20);
        assert!(aggregator::read(&aggregator) == 0, 0);

        aggregator::add(&mut aggregator, 1000);
        aggregator::sub(&mut aggregator, 1000);

        aggregator::destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 0x020001)]
    fun test_overflow(account: signer) acquires AggregatorFactory {
        initialize_aggregator_factory(&account);

        let addr = signer::address_of(&account);
        let aggregator_factory = borrow_global_mut<AggregatorFactory>(addr);

        let aggregator = new_aggregator(aggregator_factory, /*limit=*/10);

        // Overflow!
        aggregator::add(&mut aggregator, 12);

        aggregator::destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 0x020002)]
    fun test_underflow(account: signer) acquires AggregatorFactory {
        initialize_aggregator_factory(&account);

        let addr = signer::address_of(&account);
        let aggregator_factory = borrow_global_mut<AggregatorFactory>(addr);

        let aggregator = new_aggregator(aggregator_factory, /*limit=*/10);

        // Underflow!
        aggregator::sub(&mut aggregator, 100);
        aggregator::add(&mut aggregator, 100);

        aggregator::destroy(aggregator);
    }
}
