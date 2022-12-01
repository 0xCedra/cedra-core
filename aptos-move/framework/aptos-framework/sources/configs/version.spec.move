spec aptos_framework::version {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec set_version {
        use aptos_framework::chain_status;
        use aptos_framework::timestamp;

        requires chain_status::is_operating();
        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
    }

    spec set_version(account: &signer, major: u64) {
        use std::signer;

        aborts_if !exists<SetVersionCapability>(signer::address_of(account));
        aborts_if !exists<Version>(@aptos_framework);

        let old_major = global<Version>(@aptos_framework).major;
        aborts_if !(old_major < major);
    }

    /// Abort if resource already exists in `@aptos_framwork` when initializing.
    spec initialize(aptos_framework: &signer, initial_version: u64) {
        use std::signer;

        aborts_if signer::address_of(aptos_framework) != @aptos_framework;
        aborts_if exists<Version>(@aptos_framework);
        aborts_if exists<SetVersionCapability>(@aptos_framework);
    }
}
