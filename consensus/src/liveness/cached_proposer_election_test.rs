// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{cell::Cell, sync::Arc};

use aptos_infallible::Mutex;
use consensus_types::common::{Author, Round};

use crate::liveness::cached_proposer_election::CachedProposerElection;

use super::proposer_election::ProposerElection;

struct MockProposerElection {
    proposers: Vec<Author>,
    asked: Arc<Mutex<Cell<u32>>>,
}

impl MockProposerElection {
    pub fn new(proposers: Vec<Author>, asked: Arc<Mutex<Cell<u32>>>) -> Self {
        Self { proposers, asked }
    }
}

impl ProposerElection for MockProposerElection {
    fn get_valid_proposer(&self, round: Round) -> Author {
        let round_uszie = round as usize;
        let asked = self.asked.lock();
        asked.replace(asked.get() + 1);
        self.proposers[round_uszie % self.proposers.len()]
    }
}

#[test]
fn test_get_valid_proposer_caching() {
    let asked = Arc::new(Mutex::new(Cell::new(0)));
    let authors: Vec<Author> = (0..4).map(|_| Author::random()).collect();
    let cpe = CachedProposerElection::new(
        Box::new(MockProposerElection::new(authors.clone(), asked.clone())),
        10,
    );

    assert_eq!(asked.lock().get(), 0);

    assert_eq!(cpe.get_valid_proposer(0), authors[0]);
    assert_eq!(asked.lock().get(), 1);
    debug_assert!(cpe.is_valid_proposer(authors[0], 0));
    debug_assert!(!cpe.is_valid_proposer(authors[1], 0));
    assert_eq!(asked.lock().get(), 1);

    assert_eq!(cpe.get_valid_proposer(1), authors[1]);
    assert_eq!(asked.lock().get(), 2);
    debug_assert!(cpe.is_valid_proposer(authors[1], 1));
    debug_assert!(!cpe.is_valid_proposer(authors[0], 1));
    assert_eq!(asked.lock().get(), 2);

    assert_eq!(cpe.get_valid_proposer(0), authors[0]);
    assert_eq!(asked.lock().get(), 2);

    assert_eq!(cpe.get_valid_proposer(11), authors[3]);
    assert_eq!(asked.lock().get(), 3);
    debug_assert!(cpe.is_valid_proposer(authors[3], 11));
    debug_assert!(!cpe.is_valid_proposer(authors[0], 11));
    assert_eq!(asked.lock().get(), 3);

    // round=0 is outside the caching window, and round=1 is still inside
    assert_eq!(cpe.get_valid_proposer(0), authors[0]);
    assert_eq!(asked.lock().get(), 4);

    assert_eq!(cpe.get_valid_proposer(1), authors[1]);
    assert_eq!(asked.lock().get(), 4);
}
