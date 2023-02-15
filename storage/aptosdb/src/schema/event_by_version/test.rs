// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        event_key in any::<EventKey>(),
        seq_num in any::<u64>(),
        version in any::<Version>(),
        index in any::<u64>(),
    ) {
        assert_encode_decode::<EventByVersionSchema>(&(event_key, version, seq_num), &index);
    }
}

test_no_panic_decoding!(EventByVersionSchema);
