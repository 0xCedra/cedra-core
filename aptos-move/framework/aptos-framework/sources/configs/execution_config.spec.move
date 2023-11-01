spec aptos_framework::execution_config {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Ensure the caller is admin
    /// When setting now time must be later than last_reconfiguration_time.
    spec set(account: &signer, config: vector<u8>) {
        use aptos_framework::timestamp;
        use std::signer;
        use std::features;
        use aptos_framework::transaction_fee;
        use aptos_framework::chain_status;
        use aptos_framework::stake;
        use aptos_framework::staking_config;
        use aptos_framework::aptos_coin;
        use aptos_framework::reconfiguration;

        // It caused 25s to verified in the local environment and timeout in the github unit test
        let addr = signer::address_of(account);
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        include reconfiguration::ReconfigureEnsures;
        requires chain_status::is_operating();
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<staking_config::StakingRewardsConfig>(@aptos_framework);
        requires len(config) > 0;
        include features::spec_periodical_reward_rate_decrease_enabled() ==> staking_config::StakingRewardsConfigEnabledRequirement;
        include aptos_coin::ExistsAptosCoin;
        requires system_addresses::is_aptos_framework_address(addr);
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();

        ensures exists<ExecutionConfig>(@aptos_framework);
    }
}
