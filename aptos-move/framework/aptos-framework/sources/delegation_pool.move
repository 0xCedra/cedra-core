/**
 * Allow multiple delegators to participate in the same stake pool in order to collect the minimum
 * stake required to join the validator set. Delegators are rewarded out of the validator rewards
 * proportionally to their stake and provided the same stake-management API as the stake owner.
 *
 * The main accounting logic in the delegation pool contract handles the following:
 * 1. Tracks how much stake each delegator owns, privately deposited and earned too.
 * Accounting individual delegator stakes is achieved through the shares-based pool defined at
 * `aptos_std::pool_u64`, hence delegators own shares rather than absolute stakes into the delegation pool.
 * 2. Tracks rewards earned by the stake pool, implicitly by the delegation one, in the meantime
 * and distribute them accordingly.
 * 3. Tracks lockup cycles on the stake pool in order to separate inactive stake (not earning rewards)
 * from pending_inactive stake (earning rewards) and allow its delegators to withdraw it
 * 4. Tracks how much commission fee has to be paid to the operator out of incoming rewards before
 * distributing them to the internal pool_u64 pools.
 *
 * In order to distinguish between stakes in different states and route rewards accordingly,
 * separate pool_u64 pools are used for individual stake states:
 *      1. one of `active` + `pending_active` stake
 *      2. one of `inactive` stake FOR each past observed lockup cycle (OLC) detected on the stake pool
 *      3. one of `pending_inactive` stake scheduled during this ongoing OLC
 *
 * As stake-state transitions and rewards are computed only at the stake pool level, the delegation pool
 * gets outdated. To mitigate this, at any interaction with the delegation pool, a process of synchronization
 * to the underlying stake pool is executed before the requested operation itself.
 *
 * At synchronization:
 *  - stake deviations between the two pools are actually the rewards produced in the meantime.
 *  - the commission fee is extracted from the rewards, the remaining stake is distributed to the internal
 * pool_u64 pools and then commission stake used to buy shares for operator
 *  - if detecting that the lockup expired on the stake pool, the delegation pool will isolate its
 * pending_inactive stake (now inactive) and create a new pool_u64 to host future pending_inactive stake
 * at the newly started lockup.
 * Detecting a lockup expiration on the stake pool resumes to detecting new inactive stake.
 *
 * Example flow:
 * 1. A node operator creates a delegation pool by calling `initialize_delegation_pool` and sets
 * its commission fee to 0% (for simplicity). A stake pool is created with no initial stake and owned by
 * a resource account controlled by the delegation pool.
 * 2. Delegator A adds 100 stake which is converted to 100 shares into the active pool_u64
 * 3. Operator joins the validator set as the stake pool has now the minimum stake
 * 4. The stake pool earned rewards and now has 200 active stake. A's active shares are worth 200 coins as
 * the commission fee is 0%.
 * 5a. A requests `unlock` for 100 stake
 * 5b. Synchronization detects 200 - 100 active rewards which are entirely (0% commission) added to the active pool.
 * 5c. 100 coins = (100 * 100) / 200 = 50 shares are redeemed from the active pool and exchanged for 100 shares
 * into the pending_inactive one on A's behalf
 * 6. Delegator B adds 200 stake which is converted to (200 * 50) / 100 = 100 shares into the active pool
 * 7. The stake pool earned rewards and now has 600 active and 200 pending_inactive stake.
 * 8a. A requests `reactivate_stake` for 100 stake
 * 8b. Synchronization detects 600 - 300 active and 200 - 100 pending_inactive rewards which are both entirely
 * distributed to their corresponding pools
 * 8c. 100 coins = (100 * 100) / 200 = 50 shares are redeemed from the pending_inactive pool and exchanged for
 * (100 * 150) / 600 = 25 shares into the active one on A's behalf
 * 9. The lockup expires on the stake pool, inactivating the entire pending_inactive stake
 * 10a. B requests `unlock` for 100 stake
 * 10b. Synchronization detects no active or pending_inactive rewards, but 0 -> 100 inactive stake
 * on the stake pool, so it advances the observed lockup cycle and creates a pool_u64 for the new lockup,
 * hence allowing previous pending_inactive shares to be redeemed
 * 10c. 100 coins = (100 * 175) / 700 = 25 shares are redeemed from the active pool and exchanged for
 * 100 shares into the new pending_inactive one on B's behalf
 * 11. The stake pool earned rewards and now has some pending_inactive rewards.
 * 12a. A requests `withdraw` for its entire inactive stake
 * 12b. Synchronization detects no new inactive stake, but some pending_inactive rewards which are
 * distributed to the (2nd) pending_inactive pool
 * 12c. A's 50 shares = (50 * 100) / 50 = 100 coins are redeemed from the (1st) inactive pool and 100 stake
 * is transferred to A
 */
module aptos_framework::delegation_pool {
    use std::error;
    use std::signer;
    use std::vector;

    use aptos_std::math64::min;
    use aptos_std::pool_u64_unbound::{Self as pool_u64, total_coins};
    use aptos_std::table::{Self, Table};

    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::stake;
    use aptos_framework::staking_config;
    use aptos_framework::timestamp;

    const MODULE_SALT: vector<u8> = b"aptos_framework::delegation_pool";

    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    /// Delegation pool owner capability does not exist at the provided account.
    const EOWNER_CAP_NOT_FOUND: u64 = 1;

    /// Account is already owning a delegation pool.
    const EOWNER_CAP_ALREADY_EXISTS: u64 = 2;

    /// Delegation pool does not exist at the provided pool address.
    const EDELEGATION_POOL_DOES_NOT_EXIST: u64 = 3;

    /// There is a pending withdrawal to be executed before unlocking any stake
    const EPENDING_WITHDRAWAL_EXISTS: u64 = 4;

    /// Commission percentage has to be between 0 and `MAX_FEE` - 100%.
    const EINVALID_COMMISSION_PERCENTAGE: u64 = 5;

    /// Slashing (if implemented) should not be applied to already `inactive` stake.
    /// Not only it invalidates the accounting of past observed lockup cycles (OLC),
    /// but is also unfair to delegators whose stake has been inactive before validator started misbehaving.
    /// Additionally, the inactive stake does not count on the voting power of validator.
    const ESLASHED_INACTIVE_STAKE_ON_PAST_OLC: u64 = 6;

    const MAX_U64: u64 = 18446744073709551615;

    /// Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285
    const MAX_FEE: u64 = 10000;

    /// Capability that represents ownership over privileged operations on the underlying stake pool.
    struct DelegationPoolOwnership has key, store {
        /// equal to address of the resource account owning the stake pool
        pool_address: address,
    }

    struct DelegationPool has key {
        // Share pool of `active` + `pending_active` stake
        active_shares: pool_u64::Pool,
        // Index of current lockup cycle on the delegation pool since its creation
        observed_lockup_cycle: u64,
        // Share pools of `inactive` stake on each past lockup cycle and `pending_inactive` stake on current one
        inactive_shares: Table<u64, pool_u64::Pool>,
        // Unique lockup cycle (key in `inactive_shares`) where delegator has stake to withdraw
        pending_withdrawals: Table<address, u64>,
        // Signer capability of the resource account owning the stake pool
        stake_pool_signer_cap: account::SignerCapability,
        // Total (inactive) coins on the share pools over all ended lockup cycles
        total_coins_inactive: u64,
        // Commission fee paid to the node operator out of pool rewards
        operator_commission_percentage: u64,

        // The events emitted by stake-management operations on the pool
        add_stake_events: EventHandle<AddStakeEvent>,
        reactivate_stake_events: EventHandle<ReactivateStakeEvent>,
        unlock_stake_events: EventHandle<UnlockStakeEvent>,
        withdraw_stake_events: EventHandle<WithdrawStakeEvent>,
    }

    struct AddStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_added: u64,
        add_stake_fee: u64,
    }

    struct ReactivateStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount: u64,
    }

    struct UnlockStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_unlocked: u64,
    }

    struct WithdrawStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_withdrawn: u64,
    }

    public entry fun initialize_delegation_pool(
        owner: &signer,
        operator_commission_percentage: u64,
        delegation_pool_creation_seed: vector<u8>,
    ) {
        let owner_address = signer::address_of(owner);
        assert!(!owner_cap_exists(owner_address), error::already_exists(EOWNER_CAP_ALREADY_EXISTS));
        assert!(operator_commission_percentage <= MAX_FEE, error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE));

        // generate a seed to be used to create the resource account hosting the delegation pool
        let seed = vector::empty<u8>();
        // include module salt (before any subseeds) to avoid conflicts with other modules creating resource accounts
        vector::append(&mut seed, MODULE_SALT);
        // include an additional salt in case the same resource account has already been created
        vector::append(&mut seed, delegation_pool_creation_seed);

        let (stake_pool_signer, stake_pool_signer_cap) = account::create_resource_account(owner, seed);
        coin::register<AptosCoin>(&stake_pool_signer);

        // stake_pool_signer will be owner of the stake pool and have its `stake::OwnerCapability`
        let pool_address = signer::address_of(&stake_pool_signer);
        stake::initialize_stake_owner(&stake_pool_signer, 0, owner_address, owner_address);

        let inactive_shares = table::new<u64, pool_u64::Pool>();
        table::add(&mut inactive_shares, 0, pool_u64::create());

        move_to(&stake_pool_signer, DelegationPool {
            active_shares: pool_u64::create(),
            observed_lockup_cycle: 0,
            inactive_shares,
            pending_withdrawals: table::new<address, u64>(),
            stake_pool_signer_cap,
            total_coins_inactive: 0,
            operator_commission_percentage,
            add_stake_events: account::new_event_handle<AddStakeEvent>(&stake_pool_signer),
            reactivate_stake_events: account::new_event_handle<ReactivateStakeEvent>(&stake_pool_signer),
            unlock_stake_events: account::new_event_handle<UnlockStakeEvent>(&stake_pool_signer),
            withdraw_stake_events: account::new_event_handle<WithdrawStakeEvent>(&stake_pool_signer),
        });

        // save delegation pool ownership and resource account address (inner stake pool address) on `owner`
        move_to(owner, DelegationPoolOwnership { pool_address });
    }

    #[view]
    public fun owner_cap_exists(addr: address): bool {
        exists<DelegationPoolOwnership>(addr)
    }

    fun assert_owner_cap_exists(owner: address) {
        assert!(owner_cap_exists(owner), error::not_found(EOWNER_CAP_NOT_FOUND));
    }

    #[view]
    public fun get_owned_pool_address(owner: address): address acquires DelegationPoolOwnership {
        assert_owner_cap_exists(owner);
        borrow_global<DelegationPoolOwnership>(owner).pool_address
    }

    #[view]
    public fun delegation_pool_exists(addr: address): bool {
        exists<DelegationPool>(addr)
    }

    fun assert_delegation_pool_exists(pool_address: address) {
        assert!(delegation_pool_exists(pool_address), error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST));
    }

    /// Retrieves the shared resource account owning the stake pool in order
    /// to forward a stake-management operation to this underlying pool.
    fun retrieve_stake_pool_signer(pool: &DelegationPool): signer {
        account::create_signer_with_capability(&pool.stake_pool_signer_cap)
    }

    /// Get the address of delegation pool reference `pool`.
    fun get_pool_address(pool: &DelegationPool): address {
        account::get_signer_capability_address(&pool.stake_pool_signer_cap)
    }

    #[view]
    /// Return the index of current observed lockup cycle on delegation pool `pool_address`.
    /// This represents the key into `inactive_shares` of the `pending_inactive` shares pool.
    public fun observed_lockup_cycle(pool_address: address): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        borrow_global<DelegationPool>(pool_address).observed_lockup_cycle
    }

    /// Allows an owner to change the operator of the underlying stake pool.
    public entry fun set_operator(owner: &signer, new_operator: address) acquires DelegationPoolOwnership, DelegationPool {
        let pool_address = get_owned_pool_address(signer::address_of(owner));
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        stake::set_operator(&retrieve_stake_pool_signer(borrow_global<DelegationPool>(pool_address)), new_operator);
    }

    /// Allows an owner to change the delegated voter of the underlying stake pool.
    public entry fun set_delegated_voter(owner: &signer, new_voter: address) acquires DelegationPoolOwnership, DelegationPool {
        let pool_address = get_owned_pool_address(signer::address_of(owner));
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        stake::set_delegated_voter(&retrieve_stake_pool_signer(borrow_global<DelegationPool>(pool_address)), new_voter);
    }

    #[view]
    /// Return the fee to be charged for an `add_stake` operation of `amount` on pool at `pool_address`.
    /// If the validator produces rewards this epoch, added stake goes directly to `pending_active` and
    /// does not earn rewards. However, all shares within a pool_u64 appreciate uniformly, when this epoch ends:
    /// - either added shares are still `pending_active` and steal from rewards of existing `active` stake
    /// - or have moved to `pending_inactive` and get full rewards (they displaced `active` stake at `unlock`)
    /// Therefore, should charge delegator the maximum amount it would unfairly earn only this epoch.
    public fun get_add_stake_fee(pool_address: address, amount: u64): u64 {
        if (stake::is_current_epoch_validator(pool_address)) {
            let (rewards_rate, rewards_rate_denominator) = staking_config::get_reward_rate(&staking_config::get());
            if (rewards_rate_denominator > 0) {
                ((((amount as u128) * (rewards_rate as u128)) / (rewards_rate_denominator as u128)) as u64)
            } else { 0 }
        } else { 0 }
    }

    /// Add `amount` of coins to the delegation pool `pool_address`.
    public entry fun add_stake(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        // short-circuit if amount to add is 0 so no event is emitted
        if (amount == 0) { return };

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        // fee to be charged for adding `amount` stake on this delegation pool at this epoch
        let add_stake_fee = get_add_stake_fee(pool_address, amount);

        // stake the entire amount to the stake pool
        coin::transfer<AptosCoin>(delegator, pool_address, amount);
        stake::add_stake(&retrieve_stake_pool_signer(pool), amount);
        // but buy shares for delegator just for the remaining amount after fee
        buy_in_active_shares(pool, delegator_address, amount - add_stake_fee);

        // commit coins from `add_stake` fee to the active shares pool
        // in order to not mistake them for epoch active rewards
        let (active, _, pending_active, _) = stake::get_stake(pool_address);
        pool_u64::update_total_coins(&mut pool.active_shares, active + pending_active);

        event::emit_event(
            &mut pool.add_stake_events,
            AddStakeEvent {
                pool_address,
                delegator_address,
                amount_added: amount,
                add_stake_fee,
            },
        );
    }

    /// Unlock `amount` from the active + pending_active stake of `delegator` or
    /// at most how much active stake there is on the stake pool.
    public entry fun unlock(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        // short-circuit if amount to unlock is 0 so no event is emitted
        if (amount == 0) { return };

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        // capture how much stake would be unlocked on the stake pool
        let (active, _, _, _) = stake::get_stake(pool_address);
        amount = min(amount, active);
        amount = redeem_active_shares(pool, delegator_address, amount);

        stake::unlock(&retrieve_stake_pool_signer(pool), amount);
        buy_in_inactive_shares(pool, delegator_address, amount);

        event::emit_event(
            &mut pool.unlock_stake_events,
            UnlockStakeEvent {
                pool_address,
                delegator_address,
                amount_unlocked: amount,
            },
        );
    }

    /// Move `amount` of coins from pending_inactive to active.
    public entry fun reactivate_stake(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        // short-circuit if amount to reactivate is 0 so no event is emitted
        if (amount == 0) { return };

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        let observed_lockup_cycle = pool.observed_lockup_cycle;
        amount = redeem_inactive_shares(pool, delegator_address, amount, observed_lockup_cycle);

        stake::reactivate_stake(&retrieve_stake_pool_signer(pool), amount);
        buy_in_active_shares(pool, delegator_address, amount);

        event::emit_event(
            &mut pool.reactivate_stake_events,
            ReactivateStakeEvent {
                pool_address,
                delegator_address,
                amount,
            },
        );
    }

    /// Withdraw `amount` of owned inactive stake from the delegation pool at `pool_address`.
    public entry fun withdraw(delegator: &signer, pool_address: address, amount: u64) acquires DelegationPool {
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        withdraw_internal(borrow_global_mut<DelegationPool>(pool_address), signer::address_of(delegator), amount);
    }

    #[view]
    /// Return whether `pending_inactive` stake can be directly withdrawn from
    /// the delegation pool, implicitly its stake pool, in the special case
    /// the validator had gone inactive before its lockup expired.
    public fun can_withdraw_pending_inactive(pool_address: address): bool {
        stake::get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE &&
        timestamp::now_seconds() >= stake::get_lockup_secs(pool_address)
    }

    fun withdraw_internal(pool: &mut DelegationPool, delegator_address: address, amount: u64) {
        // short-circuit if amount to withdraw is 0 so no event is emitted
        if (amount == 0) { return };

        let pool_address = get_pool_address(pool);
        let (withdrawal_exists, withdrawal_lockup_cycle) = pending_withdrawal_exists(pool, delegator_address);
        // exit if no withdrawal or is not inactive and cannot withdraw pending_inactive stake from stake pool
        if (!(
            withdrawal_exists &&
            (withdrawal_lockup_cycle < pool.observed_lockup_cycle || can_withdraw_pending_inactive(pool_address))
        )) { return };

        amount = redeem_inactive_shares(pool, delegator_address, amount, withdrawal_lockup_cycle);

        let stake_pool_owner = &retrieve_stake_pool_signer(pool);
        // stake pool will inactivate entire pending_inactive stake at `stake::withdraw` to make it withdrawable
        // however, bypassing the inactivation of excess stake (inactivated but now withdrawn) ensures
        // the OLC is not advanced indefinitely on `unlock`-`withdraw` paired calls
        if (can_withdraw_pending_inactive(pool_address)) {
            // get excess stake before being entirely inactivated
            let (_, _, _, pending_inactive) = stake::get_stake(pool_address);
            if (withdrawal_lockup_cycle == pool.observed_lockup_cycle) {
                // `amount` less excess if withdrawing pending_inactive stake
                pending_inactive = pending_inactive - amount
            };
            // escape excess stake from inactivation
            stake::reactivate_stake(stake_pool_owner, pending_inactive);
            stake::withdraw(stake_pool_owner, amount);
            // restore excess stake to the pending_inactive state
            stake::unlock(stake_pool_owner, pending_inactive);
        } else {
            // no excess stake if `stake::withdraw` does not inactivate at all
            stake::withdraw(stake_pool_owner, amount);
        };
        coin::transfer<AptosCoin>(stake_pool_owner, delegator_address, amount);

        // commit withdrawal of possibly inactive stake to the `total_coins_inactive`
        // known by the delegation pool in order to not mistake it for slashing
        let (_, inactive, _, _) = stake::get_stake(pool_address);
        pool.total_coins_inactive = inactive;

        event::emit_event(
            &mut pool.withdraw_stake_events,
            WithdrawStakeEvent {
                pool_address,
                delegator_address,
                amount_withdrawn: amount,
            },
        );
    }

    #[view]
    /// Return total stake owned by `delegator_address` within delegation pool `pool_address`
    /// in each of its individual states.
    public fun get_stake(pool_address: address, delegator_address: address): (u64, u64, u64) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let (
            lockup_cycle_ended,
            active,
            pending_inactive,
            commission_active,
            commission_pending_inactive
        ) = calculate_stake_pool_drift(pool);

        active = pool_u64::shares_to_amount_with_total_coins(
            &pool.active_shares,
            pool_u64::shares(&pool.active_shares, delegator_address),
            // exclude operator active rewards not converted to shares yet
            active - commission_active
        );

        // if no pending withdrawal, there is neither inactive nor pending_inactive stake
        let (withdrawal_exists, withdrawal_lockup_cycle) = pending_withdrawal_exists(pool, delegator_address);
        let inactive;
        (inactive, pending_inactive) = if (withdrawal_exists) {
            // delegator has either inactive or pending_inactive stake due to automatic withdrawals
            let inactive_shares = table::borrow(&pool.inactive_shares, withdrawal_lockup_cycle);
            if (withdrawal_lockup_cycle < pool.observed_lockup_cycle) {
                // if withdrawal's lockup cycle was ended on delegation pool then its stake is inactive
                (pool_u64::balance(inactive_shares, delegator_address), 0)
            } else {
                pending_inactive = pool_u64::shares_to_amount_with_total_coins(
                    inactive_shares,
                    pool_u64::shares(inactive_shares, delegator_address),
                    // exclude operator pending_inactive rewards not converted to shares yet
                    pending_inactive - commission_pending_inactive
                );
                // if withdrawal's lockup cycle was ended ONLY on stake pool then its stake is inactive
                if (lockup_cycle_ended) { (pending_inactive, 0) } else { (0, pending_inactive) }
            }
        } else { (0, 0) };

        // should also include commission rewards in case of the operator account
        // operator rewards are actually used to buy shares which is introducing
        // some imprecision (received stake would be slightly less)
        // but adding rewards onto the existing stake is still a good approximation
        if (delegator_address == stake::get_operator(pool_address)) {
            active = active + commission_active;
            if (lockup_cycle_ended) {
                inactive = inactive + commission_pending_inactive;
            } else {
                pending_inactive = pending_inactive + commission_pending_inactive;
            }
        };
        (active, inactive, pending_inactive)
    }

    #[view]
    /// Return the unique observed lockup cycle where delegator `delegator_address` may have
    /// unlocking (or already unlocked) stake to be withdrawn from delegation pool at `pool_address`.
    public fun get_pending_withdrawal(pool_address: address, delegator_address: address): (bool, u64) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        pending_withdrawal_exists(borrow_global<DelegationPool>(pool_address), delegator_address)
    }

    /// Return the unique observed lockup cycle where delegator `delegator_address` may have
    /// unlocking (or already unlocked) stake to be withdrawn from delegation pool `pool`.
    /// A bool is returned to signal if a pending withdrawal exists at all.
    fun pending_withdrawal_exists(pool: &DelegationPool, delegator_address: address): (bool, u64) {
        if (table::contains(&pool.pending_withdrawals, delegator_address)) {
            (true, *table::borrow(&pool.pending_withdrawals, delegator_address))
        } else {
            (false, 0)
        }
    }

    /// Return a mutable reference to the shares pool of `pending_inactive` stake on the
    /// delegation pool, always the last item in `inactive_shares`.
    fun pending_inactive_shares_pool(pool: &mut DelegationPool): &mut pool_u64::Pool {
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        table::borrow_mut(&mut pool.inactive_shares, observed_lockup_cycle)
    }

    /// Buy shares into active pool on behalf of delegator `shareholder` who
    /// delegated `coins_amount` stake or reactivated(redeemed) it from pending-inactive pool.
    fun buy_in_active_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u64 {
        if (coins_amount == 0) return 0;

        pool_u64::buy_in(&mut pool.active_shares, shareholder, coins_amount)
    }

    /// Execute the pending withdrawal of `delegator_address` on delegation pool `pool`
    /// if existing and already inactive to allow the creation of a new one.
    /// `pending_inactive` stake would be left untouched even if withdrawable and should
    /// be explicitly withdrawn by delegator
    fun execute_pending_withdrawal(pool: &mut DelegationPool, delegator_address: address) {
        let (withdrawal_exists, withdrawal_lockup_cycle) = pending_withdrawal_exists(pool, delegator_address);
        if (withdrawal_exists && withdrawal_lockup_cycle < pool.observed_lockup_cycle) {
            withdraw_internal(pool, delegator_address, MAX_U64);
        }
    }

    /// Buy shares into pending-inactive pool on behalf of delegator `shareholder` who 
    /// redeemed `coins_amount` from active pool to schedule it for unlocking.
    /// If there is a pending withdrawal from a past cycle, fail the operation.
    fun buy_in_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u64 {
        if (coins_amount == 0) return 0;
        // execute the pending withdrawal if existing before creating a new one
        execute_pending_withdrawal(pool, shareholder);

        // save lockup cycle for new pending withdrawal if no existing previous one
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        assert!(*table::borrow_mut_with_default(
            &mut pool.pending_withdrawals,
            shareholder,
            observed_lockup_cycle
        ) == observed_lockup_cycle,
            error::invalid_state(EPENDING_WITHDRAWAL_EXISTS)
        );

        // cannot buy inactive shares, only pending_inactive at current lockup cycle
        pool_u64::buy_in(pending_inactive_shares_pool(pool), shareholder, coins_amount)
    }

    /// Convert `coins_amount` of coins to be redeemed from share pool `shares_pool`
    /// to the exact number of shares to redeem in order to achieve this.
    fun amount_to_shares_to_redeem(
        shares_pool: &pool_u64::Pool,
        shareholder: address,
        coins_amount: u64,
    ): u64 {
        if (coins_amount >= pool_u64::balance(shares_pool, shareholder)) {
            // take into account rounding errors and extract entire shares amount
            pool_u64::shares(shares_pool, shareholder)
        } else {
            pool_u64::amount_to_shares(shares_pool, coins_amount)
        }
    }

    /// Redeem shares from active pool on behalf of delegator `shareholder` who
    /// wants to unlock `coins_amount` of its active stake.
    /// Extracted coins will be used to buy shares into the pending-inactive pool and
    /// be available for redeeming when this lockup cycle ends.
    fun redeem_active_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
    ): u64 {
        let shares_to_redeem = amount_to_shares_to_redeem(&pool.active_shares, shareholder, coins_amount);
        if (shares_to_redeem == 0) return 0;
        pool_u64::redeem_shares(&mut pool.active_shares, shareholder, shares_to_redeem)
    }

    /// Redeem shares from inactive pool at `lockup_cycle` < current lockup on behalf of
    /// delegator `shareholder` who wants to withdraw `coins_amount` of its unlocked stake.
    /// Redeem shares from pending-inactive pool at `lockup_cycle` == current lockup on behalf of
    /// delegator `shareholder` who wants to reactivate `coins_amount` of its unlocking stake.
    /// For latter case, extracted coins will be used to buy shares into the active pool and
    /// escape inactivation when current lockup ends.
    fun redeem_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
        lockup_cycle: u64,
    ): u64 {
        let inactive_shares = table::borrow_mut(&mut pool.inactive_shares, lockup_cycle);
        let shares_to_redeem = amount_to_shares_to_redeem(inactive_shares, shareholder, coins_amount);
        if (shares_to_redeem == 0) return 0;
        let redeemed_coins = pool_u64::redeem_shares(inactive_shares, shareholder, shares_to_redeem);

        // if delegator entirely reactivated pending_inactive stake or withdrawn inactive stake,
        // enable unlocking again by deleting this pending withdrawal
        if (pool_u64::shares(inactive_shares, shareholder) == 0) {
            table::remove(&mut pool.pending_withdrawals, shareholder);
        };
        // destroy inactive-shares pool of past OLC if all its stake has been withdrawn
        if (lockup_cycle < pool.observed_lockup_cycle && total_coins(inactive_shares) == 0) {
            pool_u64::destroy_empty(table::remove<u64, pool_u64::Pool>(&mut pool.inactive_shares, lockup_cycle));
        };

        redeemed_coins
    }

    /// Calculate stake deviations between the delegation and stake pools in order to
    /// capture the rewards earned in the meantime, resulting operator commission and
    /// whether the lockup expired on the stake pool.
    fun calculate_stake_pool_drift(pool: &DelegationPool): (bool, u64, u64, u64, u64) {
        let (active, inactive, pending_active, pending_inactive) = stake::get_stake(get_pool_address(pool));
        assert!(
            inactive >= pool.total_coins_inactive,
            error::invalid_state(ESLASHED_INACTIVE_STAKE_ON_PAST_OLC)
        );
        // determine whether a new lockup cycle has been ended on the stake pool and
        // inactivated SOME `pending_inactive` stake which should stop earning rewards now,
        // thus requiring separation of the `pending_inactive` stake on current observed lockup
        // and the future one on the newly started lockup
        let lockup_cycle_ended = inactive > pool.total_coins_inactive;

        // actual coins on stake pool belonging to the active shares pool
        active = active + pending_active;
        // actual coins on stake pool belonging to the shares pool hosting `pending_inactive` stake
        // at current observed lockup cycle, either pending: `pending_inactive` or already inactivated:
        if (lockup_cycle_ended) {
            // `inactive` on stake pool = any previous `inactive` stake +
            // any previous `pending_inactive` stake and its rewards (both inactivated)
            pending_inactive = inactive - pool.total_coins_inactive
        };

        // on delegator operations, total coins on internal shares pools and individual
        // stakes on the stake pool are updated simultaneously, thus the only stakes becoming
        // unsynced are rewards and slashes routed directly to/out the stake pool

        // operator `active` rewards not persisted yet to the `active_shares` pool
        let commission_active = total_coins(&pool.active_shares);
        commission_active = if (active > commission_active) {
            ((active - commission_active) * pool.operator_commission_percentage) / MAX_FEE
        } else {
            // handle any slashing applied to `active` stake
            0
        };
        // operator `pending_inactive` rewards not persisted yet to the shares pool of `pending_inactive` stake
        let commission_pending_inactive = total_coins(table::borrow(&pool.inactive_shares, pool.observed_lockup_cycle));
        commission_pending_inactive = if (pending_inactive > commission_pending_inactive) {
            ((pending_inactive - commission_pending_inactive) * pool.operator_commission_percentage) / MAX_FEE
        } else {
            // handle any slashing applied to `pending_inactive` stake
            0
        };

        (lockup_cycle_ended, active, pending_inactive, commission_active, commission_pending_inactive)
    }

    /// Syncs delegation and stake pools: distribute yet-undetected rewards to corresponding internal
    /// shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.
    public entry fun synchronize_delegation_pool(pool_address: address) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let (
            lockup_cycle_ended,
            active,
            pending_inactive,
            commission_active,
            commission_pending_inactive
        ) = calculate_stake_pool_drift(pool);

        // distribute rewards remaining after commission, to delegators (to already existing shares)
        // before buying shares for the operator for its entire commission fee
        // otherwise, operator's new shares would additionally appreciate from rewards it does not own

        // update total coins accumulated by `active` + `pending_active` shares
        pool_u64::update_total_coins(&mut pool.active_shares, active - commission_active);
        // update total coins accumulated by `pending_inactive` shares at current observed lockup cycle
        pool_u64::update_total_coins(pending_inactive_shares_pool(pool), pending_inactive - commission_pending_inactive);

        // reward operator its commission out of uncommitted active rewards (`add_stake` fees already excluded)
        buy_in_active_shares(pool, stake::get_operator(pool_address), commission_active);
        // reward operator its commission out of uncommitted pending_inactive rewards
        buy_in_inactive_shares(pool, stake::get_operator(pool_address), commission_pending_inactive);

        // advance lockup cycle on delegation pool if already ended on stake pool (AND stake explicitly inactivated)
        if (lockup_cycle_ended) {
            // capture inactive coins over all ended lockup cycles (including this ending one)
            let (_, inactive, _, _) = stake::get_stake(pool_address);
            pool.total_coins_inactive = inactive;

            // advance lockup cycle on the delegation pool
            pool.observed_lockup_cycle = pool.observed_lockup_cycle + 1;
            // start new lockup cycle with a fresh shares pool for `pending_inactive` stake
            table::add(&mut pool.inactive_shares, pool.observed_lockup_cycle, pool_u64::create());
        }
    }

    #[test_only]
    use aptos_framework::reconfiguration;

    #[test_only]
    const CONSENSUS_KEY_1: vector<u8> = x"8a54b92288d4ba5073d3a52e80cc00ae9fbbc1cc5b433b46089b7804c38a76f00fc64746c7685ee628fc2d0b929c2294";
    #[test_only]
    const CONSENSUS_POP_1: vector<u8> = x"a9d6c1f1270f2d1454c89a83a4099f813a56dc7db55591d46aa4e6ccae7898b234029ba7052f18755e6fa5e6b73e235f14efc4e2eb402ca2b8f56bad69f965fc11b7b25eb1c95a06f83ddfd023eac4559b6582696cfea97b227f4ce5bdfdfed0";

    #[test_only]
    const EPOCH_DURATION: u64 = 60;
    #[test_only]
    const LOCKUP_CYCLE_SECONDS: u64 = 3600;

    #[test_only]
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    #[test_only]
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    #[test_only]
    const VALIDATOR_STATUS_PENDING_INACTIVE: u64 = 3;

    #[test_only]
    public fun end_aptos_epoch() {
        stake::end_epoch(); // additionally fast-forwards `EPOCH_DURATION` seconds
        reconfiguration::reconfigure_for_test_custom();
    }

    #[test_only]
    public fun initialize_for_test(aptos_framework: &signer) {
        initialize_for_test_custom(aptos_framework, 100, 10000, LOCKUP_CYCLE_SECONDS, true, 1, 100, 1000000);
    }

    #[test_only]
    public fun initialize_for_test_custom(
        aptos_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate_numerator: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
    ) {
        account::create_account_for_test(signer::address_of(aptos_framework));
        stake::initialize_for_test_custom(
            aptos_framework,
            minimum_stake,
            maximum_stake,
            recurring_lockup_secs,
            allow_validator_set_change,
            rewards_rate_numerator,
            rewards_rate_denominator,
            voting_power_increase_limit,
        );
        reconfiguration::initialize_for_test(aptos_framework);
    }

    #[test_only]
    public fun initialize_test_validator(
        validator: &signer,
        amount: u64,
        should_join_validator_set: bool,
        should_end_epoch: bool,
    ) acquires DelegationPoolOwnership, DelegationPool {
        let validator_address = signer::address_of(validator);
        if (!account::exists_at(validator_address)) {
            account::create_account_for_test(validator_address);
        };

        initialize_delegation_pool(validator, 0, vector::empty<u8>());

        // validator is initially stake pool's operator and voter
        let pool_address = get_owned_pool_address(validator_address);
        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1, CONSENSUS_POP_1);

        if (amount > 0) {
            stake::mint(validator, amount);
            add_stake(validator, pool_address, amount);
        };

        if (should_join_validator_set) {
            stake::join_validator_set(validator, pool_address);
        };

        if (should_end_epoch) {
            end_aptos_epoch();
        };
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_set_operator_and_delegated_voter(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);

        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(validator, 0, vector::empty<u8>());
        let pool_address = get_owned_pool_address(validator_address);

        assert!(stake::get_operator(pool_address) == @0x123, 1);
        assert!(stake::get_delegated_voter(pool_address) == @0x123, 1);

        set_operator(validator, @0x111);
        assert!(stake::get_operator(pool_address) == @0x111, 2);

        set_delegated_voter(validator, @0x112);
        assert!(stake::get_delegated_voter(pool_address) == @0x112, 2);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun test_cannot_set_operator(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        // account does not own any delegation pool
        set_operator(validator, @0x111);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun test_cannot_set_delegated_voter(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        // account does not own any delegation pool
        set_delegated_voter(validator, @0x112);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x80002, location = Self)]
    public entry fun test_already_owns_delegation_pool(
        aptos_framework: &signer,
        validator: &signer,
    ) {
        initialize_for_test(aptos_framework);
        initialize_delegation_pool(validator, 0, x"00");
        initialize_delegation_pool(validator, 0, x"01");
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_initialize_delegation_pool(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);

        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(validator, 0, vector::empty<u8>());
        let pool_address = get_owned_pool_address(validator_address);

        assert!(stake::stake_pool_exists(pool_address), 1);
        assert!(stake::get_operator(pool_address) == validator_address, 2);
        assert!(stake::get_delegated_voter(pool_address) == validator_address, 3);

        assert_delegation_pool_exists(pool_address);
        assert_owner_cap_exists(validator_address);
        assert!(borrow_global<DelegationPoolOwnership>(validator_address).pool_address == pool_address, 4);

        let (consensus_pubkey, network_addresses, fullnode_addresses) = stake::get_validator_config(pool_address);
        assert!(consensus_pubkey == vector::empty<u8>(), 7);
        assert!(network_addresses == vector::empty<u8>(), 8);
        assert!(fullnode_addresses == vector::empty<u8>(), 9);

        assert!(observed_lockup_cycle(pool_address) == 0, 10);
        stake::assert_stake_pool(pool_address, 0, 0, 0, 0);
    }

    #[test_only]
    public fun add_stake_events_counter(pool_address: address): u64 acquires DelegationPool {
        event::counter<AddStakeEvent>(&borrow_global<DelegationPool>(pool_address).add_stake_events)
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_add_stake_fee(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test_custom(aptos_framework, 100, 10000000, LOCKUP_CYCLE_SECONDS, true, 1, 100, 1000000000);
        initialize_test_validator(validator, 1000, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // entire stake on delegation pool is active
        let previous_add_stake_fee = get_add_stake_fee(pool_address, 1000);
        // `add_stake` fee: 1000 * 1%
        assert!(previous_add_stake_fee == 10, 0);
        end_aptos_epoch();
        // still entire stake on delegation pool is active
        assert!(get_add_stake_fee(pool_address, 1000) == previous_add_stake_fee, 0);

        // add pending_active stake and check fee
        stake::mint(delegator, 1000);
        add_stake(delegator, pool_address, 1000);
        stake::assert_stake_pool(pool_address, 1010, 0, 1000, 0);
        // `add_stake` fee: 1000 * 1%
        assert!(get_add_stake_fee(pool_address, 1000) == 10, 0);
        end_aptos_epoch();
        // after epoch ends should receive at most what initially deposited
        let (delegator_active, _, _) = get_stake(pool_address, delegator_address);
        assert!(delegator_active <= 1000, 0);

        // add pending_active stake from other account
        stake::mint(validator, 1000);
        add_stake(validator, pool_address, 1000);

        // add more pending_active stake over existing one and check fee
        stake::mint(delegator, 1000);
        add_stake(delegator, pool_address, 1000);
        stake::assert_stake_pool(pool_address, 2020, 0, 2000, 0);
        // `add_stake` fee: 1000 * 1%
        assert!(get_add_stake_fee(pool_address, 1000) == 10, 0);
        end_aptos_epoch();
        // after epoch ends should receive at most what initially deposited
        (delegator_active, _, _) = get_stake(pool_address, delegator_address);
        assert!(delegator_active <= 1010 + 1000, delegator_active);

        // add pending_active stake from other account
        stake::mint(validator, 1000);
        add_stake(validator, pool_address, 1000);
        // remove active stake from other account
        unlock(validator, pool_address, 1500);

        // add pending_active stake after active stake has decreased
        stake::mint(delegator, 1000);
        add_stake(delegator, pool_address, 1000);
        end_aptos_epoch();
        // after epoch ends should receive at most what initially deposited
        (delegator_active, _, _) = get_stake(pool_address, delegator_address);
        assert!(delegator_active <= 2030 + 1000, delegator_active);

        // add a larger amount of pending_active stake compared to the active one
        stake::mint(delegator, 1000000);
        add_stake(delegator, pool_address, 1000000);
        end_aptos_epoch();
        // after epoch ends should receive at most what initially deposited
        (delegator_active, _, _) = get_stake(pool_address, delegator_address);
        assert!(delegator_active <= 3060 + 1000000, delegator_active);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_add_stake_single(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, false, false);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // stake pool is pending active => all stake is activated by default
        stake::assert_stake_pool(pool_address, 1000, 0, 0, 0);
        assert_delegation(validator_address, pool_address, 1000, 0, 0);

        stake::mint(validator, 250);
        assert!(coin::balance<AptosCoin>(validator_address) == 250, 0);
        // zero `add_stake` fee as validator is not in the validator set
        assert!(get_add_stake_fee(pool_address, 250) == 0, 0);

        let add_stake_events = add_stake_events_counter(pool_address);
        // check `add_stake` increases delegator and stake pool active balances
        add_stake(validator, pool_address, 250);
        // check coins have been transferred out of delegator account
        assert!(coin::balance<AptosCoin>(validator_address) == 0, 0);
        // check a new `add_stake` event has been emitted
        assert!(add_stake_events_counter(pool_address) == add_stake_events + 1, 0);
        // zero `add_stake` fee as pool will not produce rewards this epoch
        assert_delegation(validator_address, pool_address, 1250, 0, 0);
        // added stake is still activated directly on the stake pool
        stake::assert_stake_pool(pool_address, 1250, 0, 0, 0);

        // check that no event is emitted if added stake is 0
        add_stake_events = add_stake_events_counter(pool_address);
        add_stake(validator, pool_address, 0);
        assert!(add_stake_events_counter(pool_address) == add_stake_events, 0);

        // move validator to active state
        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        // add 250 coins being pending active until next epoch
        stake::mint(validator, 250);
        // `add_stake` fee: 250 * 1%
        assert!(get_add_stake_fee(pool_address, 250) == 2, 0);
        add_stake(validator, pool_address, 250);
        // zero `add_stake` fee as there is only one delegator which gets back the paid fee
        // as entire active stake is owned by itself
        assert_delegation(validator_address, pool_address, 1500, 0, 0);
        stake::assert_stake_pool(pool_address, 1250, 0, 250, 0);

        // add 100 additional coins being pending active until next epoch
        stake::mint(validator, 100);
        add_stake(validator, pool_address, 100);
        stake::assert_stake_pool(pool_address, 1250, 0, 350, 0);
        assert_delegation(validator_address, pool_address, 1600, 0, 0);

        end_aptos_epoch();
        // 1250 active stake (* 1% rewards) + 350 pending active stake
        assert_delegation(validator_address, pool_address, 1612, 0, 0);
        stake::assert_stake_pool(pool_address, 1612, 0, 0, 0);

        // add stake which should produce no rewards yet
        stake::mint(validator, 200);
        add_stake(validator, pool_address, 200);

        end_aptos_epoch();
        // 1612 active stake (* 1% rewards) + 200 pending active stake
        assert_delegation(validator_address, pool_address, 1828, 0, 0);

        // add 1 Aptos unit stake
        stake::mint(validator, 1);
        add_stake_events = add_stake_events_counter(pool_address);
        add_stake(validator, pool_address, 1);

        assert!(get_add_stake_fee(pool_address, 1) == 0, 0);
        // event is emitted as initial added stake still reaches the stake pool
        assert!(add_stake_events_counter(pool_address) == add_stake_events + 1, 0);
        assert_delegation(validator_address, pool_address, 1829, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_add_stake_many(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::assert_stake_pool(pool_address, 1000, 0, 0, 0);

        // add 250 coins from another account
        stake::mint(delegator, 250);
        // `add_stake` fee: 250 * 1%
        assert!(get_add_stake_fee(pool_address, 250) == 2, 0);
        add_stake(delegator, pool_address, 250);
        assert_delegation(delegator_address, pool_address, 248, 0, 0);
        stake::assert_stake_pool(pool_address, 1000, 0, 250, 0);

        end_aptos_epoch();
        // 1000 active stake * 1% rewards
        stake::assert_stake_pool(pool_address, 1260, 0, 0, 0);

        // 2 `add_stake` fee + 1000 active stake * 1% = 12 total rewards
        // 12 * 248 / (1000 + 248) = 2
        assert_delegation(delegator_address, pool_address, 250, 0, 0);
        // 12 * 1000 / (1000 + 248) = 9
        assert_delegation(validator_address, pool_address, 1009, 0, 0);

        stake::mint(validator, 250);
        // `add_stake` fee: 250 * 1%
        assert!(get_add_stake_fee(pool_address, 250) == 2, 0);
        add_stake(validator, pool_address, 250);
        assert_delegation(validator_address, pool_address, 1259, 0, 0);
        assert_delegation(delegator_address, pool_address, 250, 0, 0);
        stake::assert_stake_pool(pool_address, 1260, 0, 250, 0);

        stake::mint(delegator, 100);
        // `add_stake` fee: 100 * 1%
        assert!(get_add_stake_fee(pool_address, 100) == 1, 0);
        add_stake(delegator, pool_address, 100);
        assert_delegation(delegator_address, pool_address, 349, 0, 0);
        assert_delegation(validator_address, pool_address, 1260, 0, 0);
        stake::assert_stake_pool(pool_address, 1260, 0, 350, 0);

        end_aptos_epoch();
        // 249 active stake * 1% rewards + 100 pending active
        assert_delegation(delegator_address, pool_address, 351, 0, 0);
        // 1010 active stake * 1% rewards + 250 pending active
        assert_delegation(validator_address, pool_address, 1270, 0, 0);
        stake::assert_stake_pool(pool_address, 1622, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_unlock_single(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 100, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add 200 coins being pending active until next epoch
        stake::mint(validator, 200);
        add_stake(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 300, 0, 0);
        stake::assert_stake_pool(pool_address, 100, 0, 200, 0);

        // cannot unlock pending active stake of pool (100/300 stake can move to pending inactive)
        unlock(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 201, 0, 99);
        stake::assert_stake_pool(pool_address, 1, 0, 200, 99);
        let (withdrawal_exists, withdrawal_lockup_cycle) = get_pending_withdrawal(pool_address, validator_address);
        assert!(withdrawal_exists && withdrawal_lockup_cycle == 0, 0);

        // reactivate entire pending inactive stake progressively
        reactivate_stake(validator, pool_address, 50);
        assert_delegation(validator_address, pool_address, 251, 0, 49);
        (withdrawal_exists, withdrawal_lockup_cycle) = get_pending_withdrawal(pool_address, validator_address);
        assert!(withdrawal_exists && withdrawal_lockup_cycle == 0, 0);

        reactivate_stake(validator, pool_address, 50);
        assert_delegation(validator_address, pool_address, 300, 0, 0);
        (withdrawal_exists, _) = get_pending_withdrawal(pool_address, validator_address);
        assert!(!withdrawal_exists, 0);

        stake::assert_stake_pool(pool_address, 100, 0, 200, 0);

        end_aptos_epoch();
        // 100 active stake * 1% rewards + 200 pending active
        assert_delegation(validator_address, pool_address, 301, 0, 0);
        stake::assert_stake_pool(pool_address, 301, 0, 0, 0);

        // once stake becomes active can unlock any amount of it (expect some imprecision when moving shares)
        unlock(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 152, 0, 149);
        stake::assert_stake_pool(pool_address, 152, 0, 0, 149);
        (withdrawal_exists, withdrawal_lockup_cycle) = get_pending_withdrawal(pool_address, validator_address);
        assert!(withdrawal_exists && withdrawal_lockup_cycle == 0, 0);

        assert!(stake::get_remaining_lockup_secs(pool_address) == LOCKUP_CYCLE_SECONDS - EPOCH_DURATION, 1);
        end_aptos_epoch(); // forwards EPOCH_DURATION seconds

        // pending inactive stake should not be inactivated yet
        // 152 active stake * 1% + 149 pending inactive stake * 1% rewards
        assert_delegation(validator_address, pool_address, 153, 0, 150);
        stake::assert_stake_pool(pool_address, 153, 0, 0, 150);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - 2 * EPOCH_DURATION);
        end_aptos_epoch(); // forwards another EPOCH_DURATION and unlocks stake

        // 153 active stake * 1% + 150 pending inactive stake * 1% rewards
        assert_delegation(validator_address, pool_address, 154, 151, 0);

        // add 50 coins from another account
        stake::mint(delegator, 50);
        add_stake(delegator, pool_address, 50);
        // lockup cycle on delegation pool should have changed on `add_stake` operation
        assert!(observed_lockup_cycle(pool_address) == 1, 0);

        // `add_stake` fee: 50 * 1% = 0, but there is some imprecision when buying shares
        assert_delegation(delegator_address, pool_address, 49, 0, 0);
        assert_delegation(validator_address, pool_address, 154, 151, 0);
        stake::assert_stake_pool(pool_address, 154, 151, 50, 0);

        // try to withdraw stake unlocked by others
        withdraw(delegator, pool_address, 50);
        assert!(coin::balance<AptosCoin>(delegator_address) == 0, 1);

        // withdraw own unlocked stake
        withdraw(validator, pool_address, 151);
        assert!(coin::balance<AptosCoin>(validator_address) == 151, 2);
        assert_delegation(validator_address, pool_address, 154, 0, 0);
        // pending withdrawal has been executed and deleted
        (withdrawal_exists, _) = get_pending_withdrawal(pool_address, validator_address);
        assert!(!withdrawal_exists, 0);
        // entire inactive stake on lockup cycle 0 has been withdrawn => shares pool deleted
        assert!(!table::contains(&borrow_global<DelegationPool>(pool_address).inactive_shares, 0), 0);

        // new pending withdrawal can be created on lockup cycle 1
        unlock(validator, pool_address, 55);
        assert_delegation(validator_address, pool_address, 100, 0, 54);
        (withdrawal_exists, withdrawal_lockup_cycle) = get_pending_withdrawal(pool_address, validator_address);
        assert!(withdrawal_exists && withdrawal_lockup_cycle == 1, 0);

        // end lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - EPOCH_DURATION);
        end_aptos_epoch();
        assert_delegation(validator_address, pool_address, 100, 54, 0);

        // try to unlock when there is already a pending withdrawal which is automatically executed
        unlock(validator, pool_address, 100);
        assert!(coin::balance<AptosCoin>(validator_address) == 205, 2);
        assert_delegation(validator_address, pool_address, 0, 0, 100);
        // this is the new pending withdrawal replacing the old unlocked one
        (withdrawal_exists, withdrawal_lockup_cycle) = get_pending_withdrawal(pool_address, validator_address);
        assert!(withdrawal_exists && withdrawal_lockup_cycle == 2, 0);

        // dummy validator just to be able to leave validator set with the other one
        initialize_test_validator(delegator, 100, true, true);
        // validator leaves validator set
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();
        assert!(!stake::is_current_epoch_validator(pool_address), 0);

        // end lockup cycle 2
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - EPOCH_DURATION);
        end_aptos_epoch();

        // stake remains pending inactive as it has not been explicitly inactivated
        stake::assert_stake_pool(pool_address, 51, 0, 0, 102);
        assert_delegation(validator_address, pool_address, 0, 0, 102);

        // however, as validator is inactive and lockup expired, this stake is withdrawable
        withdraw(validator, pool_address, 102);
        assert!(coin::balance<AptosCoin>(validator_address) == 307, 2);
        assert_delegation(validator_address, pool_address, 0, 0, 0);
        stake::assert_stake_pool(pool_address, 51, 0, 0, 0);
        (withdrawal_exists, _) = get_pending_withdrawal(pool_address, validator_address);
        assert!(!withdrawal_exists, 0);

        // shares pool of `pending_inactive` stake has 0 coins now
        assert!(pool_u64::total_coins(table::borrow(&borrow_global<DelegationPool>(pool_address).inactive_shares, 2)) == 0, 0);
        // but has not been deleted as is not immutable yet (others can still `unlock` stake)
        assert!(table::contains(&borrow_global<DelegationPool>(pool_address).inactive_shares, 2), 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_total_coins_inactive(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 100);
        stake::mint(delegator2, 200);
        add_stake(delegator1, pool_address, 100);
        add_stake(delegator2, pool_address, 200);
        assert_delegation(delegator1_address, pool_address, 99, 0, 0);
        assert_delegation(delegator2_address, pool_address, 198, 0, 0);
        end_aptos_epoch();

        // unlock some stake from delegator1
        unlock(delegator1, pool_address, 50);
        assert_delegation(delegator1_address, pool_address, 50, 0, 49);

        // move to lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - EPOCH_DURATION);
        end_aptos_epoch();

        synchronize_delegation_pool(pool_address);
        assert!(observed_lockup_cycle(pool_address) == 1, 0);
        assert!(borrow_global<DelegationPool>(pool_address).total_coins_inactive == 49, 0);

        // delegator1's pending_inactive stake has been inactivated
        assert_delegation(delegator1_address, pool_address, 51, 49, 0);

        // unlock some stake from delegator2
        unlock(delegator2, pool_address, 50);
        assert_delegation(delegator2_address, pool_address, 152, 0, 49);

        // withdraw some of inactive stake for delegator1
        withdraw(delegator1, pool_address, 20);
        assert_delegation(delegator1_address, pool_address, 51, 29, 0);
        assert!(borrow_global<DelegationPool>(pool_address).total_coins_inactive == 29, 0);

        // move to lockup cycle 2
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - EPOCH_DURATION);
        let (_, inactive, _, pending_inactive) = stake::get_stake(pool_address);
        end_aptos_epoch();
        // pending_inactive stake has been inactivated
        assert_delegation(delegator2_address, pool_address, 153, 49, 0);
        // inactive stake on delegation pool remains unchanged in absence of user operations
        assert!(borrow_global<DelegationPool>(pool_address).total_coins_inactive == inactive, 0);

        synchronize_delegation_pool(pool_address);
        assert!(observed_lockup_cycle(pool_address) == 2, 0);
        assert!(borrow_global<DelegationPool>(pool_address).total_coins_inactive == inactive + pending_inactive, 0);

        // withdraw some of inactive stake for delegator2
        withdraw(delegator2, pool_address, 30);
        (_, inactive, _, _) = stake::get_stake(pool_address);
        assert!(borrow_global<DelegationPool>(pool_address).total_coins_inactive == inactive, 0);

        // unlock some stake from validator
        unlock(validator, pool_address, 50);
        (_, _, _, pending_inactive) = stake::get_stake(pool_address);

        // dummy validator just to be able to leave validator set with the other one
        initialize_test_validator(delegator1, 100, true, true);
        // validator leaves validator set
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();

        // move to lockup cycle 3
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - EPOCH_DURATION);
        end_aptos_epoch();
        synchronize_delegation_pool(pool_address);

        // no pending_inactive stake has been inactivated this lockup cycle
        let (_, inactive_now, _, pending_inactive_now) = stake::get_stake(pool_address);
        assert!(inactive_now == inactive, inactive_now);
        assert!(pending_inactive_now == pending_inactive, pending_inactive_now);
        assert!(borrow_global<DelegationPool>(pool_address).total_coins_inactive == inactive, 0);

        // withdraw all inactive and pending inactive stake
        withdraw(delegator1, pool_address, MAX_U64);
        withdraw(delegator2, pool_address, MAX_U64);
        withdraw(validator, pool_address, MAX_U64);
        assert!(borrow_global<DelegationPool>(pool_address).total_coins_inactive == 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_reactivate_stake_single(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200, true, true);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // unlock some stake from the active one
        unlock(validator, pool_address, 100);
        assert_delegation(validator_address, pool_address, 100, 0, 100);
        stake::assert_stake_pool(pool_address, 100, 0, 0, 100);

        // add some stake in pending active state
        stake::mint(validator, 150);
        add_stake(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 250, 0, 100);
        stake::assert_stake_pool(pool_address, 100, 0, 150, 100);

        // can reactivate only pending inactive stake
        reactivate_stake(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 350, 0, 0);
        stake::assert_stake_pool(pool_address, 200, 0, 150, 0);

        end_aptos_epoch();
        // 200 active stake * 1% rewards + 150 pending active
        assert_delegation(validator_address, pool_address, 352, 0, 0);

        // unlock stake added at previous epoch (expect some imprecision when moving shares)
        unlock(validator, pool_address, 150);
        assert_delegation(validator_address, pool_address, 203, 0, 149);
        stake::assert_stake_pool(pool_address, 203, 0, 0, 149);

        // inactivate the pending inactive stake
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - 2 * EPOCH_DURATION);
        end_aptos_epoch();
        // 203 active stake * 1% rewards and 149 pending inactive stake * 1% rewards
        assert_delegation(validator_address, pool_address, 205, 150, 0);

        // try to unlock stake in a new lockup cycle (pending withdrawal is executed)
        unlock(validator, pool_address, 100);
        assert!(coin::balance<AptosCoin>(validator_address) == 150, 1);
        assert_delegation(validator_address, pool_address, 106, 0, 99);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_active_stake_rewards(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        // 1000 active stake * 1% rewards
        assert_delegation(validator_address, pool_address, 1010, 0, 0);

        // add stake in pending_active state
        stake::mint(validator, 200);
        add_stake(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 1210, 0, 0);

        end_aptos_epoch();
        // 1010 active stake * 1% rewards and 200 pending_active stake with no rewards yet
        assert_delegation(validator_address, pool_address, 1220, 0, 0);
        end_aptos_epoch();

        // 1220 active stake * 1% rewards
        assert_delegation(validator_address, pool_address, 1232, 0, 0);

        // 1232 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1244 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1256 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1268 active stake * 1% rewards == 12
        end_aptos_epoch();
        // 1280 active stake * 1% rewards == 12
        end_aptos_epoch();
        assert_delegation(validator_address, pool_address, 1292, 0, 0);

        // schedule some coins for unlocking (expect some imprecision when moving shares)
        unlock(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 1093, 0, 199);

        // end this lockup cycle
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        // 1093 active stake * 1% rewards and 199 pending_inactive * 1% rewards
        assert_delegation(validator_address, pool_address, 1103, 200, 0);

        end_aptos_epoch();
        // 1103 active stake * 1% rewards and 0 from inactive stake
        assert_delegation(validator_address, pool_address, 1114, 200, 0);

        // add stake in pending_active state
        stake::mint(validator, 1000);
        add_stake(validator, pool_address, 1000);
        assert_delegation(validator_address, pool_address, 2114, 200, 0);

        end_aptos_epoch();
        // 1114 active stake * 1% rewards and 0 from inactive stake
        assert_delegation(validator_address, pool_address, 2125, 200, 0);

        end_aptos_epoch();
        // 2125 active stake * 1% rewards
        assert_delegation(validator_address, pool_address, 2146, 200, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_active_stake_rewards_multiple(
        aptos_framework: &signer,
        validator: &signer,
        delegator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 200, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add stake in pending_active state
        stake::mint(delegator, 300);
        add_stake(delegator, pool_address, 300);
        // after `add_stake` fee: 300 * 200 / (200 + 200 active * 1%) = 297
        // 3 `add_stake` fee * 297 / (200 + 297) = 1
        assert_delegation(delegator_address, pool_address, 298, 0, 0);
        // 3 `add_stake` fee * 200 / (200 + 297) = 1
        assert_delegation(validator_address, pool_address, 201, 0, 0);
        stake::assert_stake_pool(pool_address, 200, 0, 300, 0);

        // 200 active stake * 1% rewards are shared between active and pending_active stakes for this epoch only
        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 299, 0, 0);
        assert_delegation(validator_address, pool_address, 202, 0, 0);
        stake::assert_stake_pool(pool_address, 502, 0, 0, 0);

        // from now delegators earn their own rewards
        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 302, 0, 0);
        assert_delegation(validator_address, pool_address, 204, 0, 0);
        stake::assert_stake_pool(pool_address, 507, 0, 0, 0);

        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 305, 0, 0);
        assert_delegation(validator_address, pool_address, 206, 0, 0);
        stake::assert_stake_pool(pool_address, 512, 0, 0, 0);

        end_aptos_epoch();
        assert_delegation(delegator_address, pool_address, 308, 0, 0);
        assert_delegation(validator_address, pool_address, 208, 0, 0);
        stake::assert_stake_pool(pool_address, 517, 0, 0, 0);

        // add more stake in pending_active state than currently active
        stake::mint(delegator, 1000);
        add_stake(delegator, pool_address, 1000);
        // after `add_stake` fee: 1000 * 517 / (517 + 517 active * 1%) = 990
        // 10 `add_stake` fee * (308 + 990) / (308 + 990 + 208) ~ 9
        assert_delegation(delegator_address, pool_address, 1307, 0, 0);
        // 10 `add_stake` fee * 208 / (308 + 990 + 208) ~ 1
        assert_delegation(validator_address, pool_address, 209, 0, 0);

        end_aptos_epoch();
        // delegator should have at most 308 + 308 active * 1% + 1000 stake
        assert_delegation(delegator_address, pool_address, 1311, 0, 0);
        assert_delegation(validator_address, pool_address, 210, 0, 0);
        stake::assert_stake_pool(pool_address, 1522, 0, 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123)]
    public entry fun test_pending_inactive_stake_rewards(
        aptos_framework: &signer,
        validator: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        assert_delegation(validator_address, pool_address, 1010, 0, 0);

        // schedule some coins for unlocking (expect some imprecision when moving shares)
        unlock(validator, pool_address, 200);
        assert_delegation(validator_address, pool_address, 811, 0, 199);

        end_aptos_epoch(); // 811 * 1% active and 199 * 1% pending inactive
        end_aptos_epoch(); // 819 * 1% active and 200 * 1% pending inactive

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch(); // 827 * 1% active and 202 * 1% pending inactive
        end_aptos_epoch(); // 835 * 1% active and 0 pending inactive (inactivated)
        assert_delegation(validator_address, pool_address, 843, 204, 0);

        // triggers an automatic execution of the unlocked pending withdraw
        unlock(validator, pool_address, 200);
        assert!(coin::balance<AptosCoin>(validator_address) == 204, 1);
        assert_delegation(validator_address, pool_address, 644, 0, 199);

        end_aptos_epoch(); // 644 * 1% active and 199 * 1% pending inactive
        end_aptos_epoch(); // 650 * 1% active and 200 * 1% pending inactive
        end_aptos_epoch(); // 656 * 1% active and 202 * 1% pending inactive
        end_aptos_epoch(); // 662 * 1% active and 204 * 1% pending inactive
        // the lockup cycle is not ended, pending inactive stake still earning
        assert_delegation(validator_address, pool_address, 668, 0, 206);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_out_of_order_redeem(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test(aptos_framework);
        initialize_test_validator(validator, 1000, true, true);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 300);
        add_stake(delegator1, pool_address, 300);

        stake::mint(delegator2, 300);
        add_stake(delegator2, pool_address, 300);

        end_aptos_epoch();
        assert!(observed_lockup_cycle(pool_address) == 0, 0);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // create the pending withdrawal for delegator 1 in lockup cycle 0
        unlock(delegator1, pool_address, 150);
        assert!(observed_lockup_cycle(pool_address) == 0, 0);

        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // create the pending withdrawal for delegator 2 in lockup cycle 1
        unlock(delegator2, pool_address, 150);
        assert!(observed_lockup_cycle(pool_address) == 1, 0);

        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        // move to lockup cycle 2 as there was some stake to inactivate
        end_aptos_epoch();

        // both delegators unlocking at different lockup cycles should be able to withdraw their stakes
        let (_, inactive, _) = get_stake(pool_address, delegator1_address);
        withdraw(delegator1, pool_address, inactive);
        withdraw(delegator2, pool_address, 150);
        assert!(observed_lockup_cycle(pool_address) == 2, 0);

        assert!(coin::balance<AptosCoin>(delegator1_address) > 0, 0);
        assert!(coin::balance<AptosCoin>(delegator2_address) > 0, 0);
    }

    #[test(aptos_framework = @aptos_framework, validator = @0x123, delegator1 = @0x010, delegator2 = @0x020)]
    public entry fun test_operator_fee(
        aptos_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer,
    ) acquires DelegationPoolOwnership, DelegationPool {
        initialize_for_test_custom(aptos_framework, 100, 100000, LOCKUP_CYCLE_SECONDS, true, 1, 100, 10000000);
        let validator_address = signer::address_of(validator);
        account::create_account_for_test(validator_address);

        // create delegation pool of commission fee 20%
        initialize_delegation_pool(validator, 2000, vector::empty<u8>());

        // validator is initially stake pool's operator
        let pool_address = get_owned_pool_address(validator_address);
        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1, CONSENSUS_POP_1);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 10000);
        add_stake(delegator1, pool_address, 10000);

        stake::mint(delegator2, 20000);
        add_stake(delegator2, pool_address, 20000);

        // activate validator
        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30000, 0, 0, 0);

        // produce active rewards
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30300, 0, 0, 0);
        // 300 active * 20% rewards
        assert_delegation(validator_address, pool_address, 60, 0, 0);
        // 100 active * 80% rewards
        assert_delegation(delegator1_address, pool_address, 10080, 0, 0);
        // 200 active * 80% rewards
        assert_delegation(delegator2_address, pool_address, 20160, 0, 0);

        unlock(delegator2, pool_address, 10000);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 20504, 0, 0, 10098);
        // 203 active * 20% and 99 pending_inactive * 20% rewards
        assert_delegation(validator_address, pool_address, 99, 0, 19);
        // 100 active * 80% rewards
        assert_delegation(delegator1_address, pool_address, 10161, 0, 0);
        // 100 active * 80% and 100 pending_inactive * 80% rewards
        assert_delegation(delegator2_address, pool_address, 10242, 0, 10079);

        unlock(delegator2, pool_address, 10000);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10610, 20297, 0, 0);
        // 105 active * 20% and 200 pending_inactive * 20% rewards
        // operator's accumulated rewards from pending_inactive stake have been also inactivated
        assert_delegation(validator_address, pool_address, 121, 58, 0);
        // 2 active * 80% and 200 pending_inactive * 80% rewards
        assert_delegation(delegator2_address, pool_address, 244, 20238, 0);

        withdraw(delegator2, pool_address, MAX_U64);
        // operator rewards have been persisted and it can be noticed there is a small imprecision
        // in computing real-time stake using `get_stake` in the operator case
        assert_delegation(validator_address, pool_address, 120, 58, 0);

        stake::mint(delegator1, 10000);
        assert!(get_add_stake_fee(pool_address, 10000) == 100, 0);
        add_stake(delegator1, pool_address, 10000);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 20716, 58, 0, 0);

        // 106 active * 20% and 121 active stake * 1% rewards and no commission from 99 `add_stake` fees
        assert_delegation(validator_address, pool_address, 142, 58, 0);
        set_operator(validator, delegator2_address);

        end_aptos_epoch();
        // old operator stopped being rewarded starting from previous epoch
        // 147 active stake * 1% rewards
        assert_delegation(validator_address, pool_address, 143, 58, 0);
    }

    #[test_only]
    public fun assert_delegation(
        delegator_address: address,
        pool_address: address,
        active_stake: u64,
        inactive_stake: u64,
        pending_inactive_stake: u64,
    ) acquires DelegationPool {
        let (actual_active, actual_inactive, actual_pending_inactive) = get_stake(pool_address, delegator_address);
        assert!(actual_active == active_stake, actual_active);
        assert!(actual_inactive == inactive_stake, actual_inactive);
        assert!(actual_pending_inactive == pending_inactive_stake, actual_pending_inactive);
    }
}