// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{TxnIndex, VersionedModule};
use crossbeam::utils::CachePadded;
use dashmap::{mapref::entry::Entry, DashMap};
use hashbrown::HashMap;
use move_vm_types::code::ModuleCache;
use std::{hash::Hash, mem, ops::Deref, sync::Arc};

/// A per-block module cache to be used for parallel transaction execution.
pub struct SyncModuleCache<K, M> {
    cache: DashMap<K, CachePadded<Arc<VersionedModule<M>>>>,
}

impl<K, M> SyncModuleCache<K, M>
where
    K: Eq + Hash + Clone,
{
    /// Returns a new empty module cache.
    pub(crate) fn empty() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Returns the number of modules stored in cache.
    pub fn num_modules(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the module cache contains an entry.
    pub fn contains(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }

    /// Returns the version of the module the cache contains. Returns [None] if cache does not have
    /// the module.
    pub fn get_module_version(&self, key: &K) -> Option<Option<TxnIndex>> {
        self.cache.get(key).map(|module| module.version())
    }

    pub(crate) fn take_modules_iter(&mut self) -> impl Iterator<Item = (K, M)> {
        // TODO(loader_v2): Use panic error instead?
        mem::take(&mut self.cache)
            .into_iter()
            .map(|(key, versioned_module)| {
                (
                    key,
                    Arc::into_inner(versioned_module.into_inner())
                        .expect("Should be uniquely owned")
                        .into_module(),
                )
            })
    }

    pub fn filter_into<T, P, F>(&self, collector: &mut HashMap<K, T>, p: P, f: F)
    where
        P: Fn(&M) -> bool,
        F: Fn(&M) -> T,
    {
        for r in self.cache.iter().filter(|r| p(r.value())) {
            collector.insert(r.key().clone(), f(r.value()));
        }
    }
}

impl<K, M> ModuleCache for SyncModuleCache<K, M>
where
    K: Eq + Hash + Clone,
{
    type Key = K;
    type Module = VersionedModule<M>;

    fn insert_module(&self, key: Self::Key, module: Self::Module) {
        self.cache.insert(key, CachePadded::new(Arc::new(module)));
    }

    fn get_module_or_insert_with<F, E>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Arc<Self::Module>>, E>
    where
        F: FnOnce() -> Result<Option<Self::Module>, E>,
    {
        if let Some(m) = self.cache.get(key) {
            return Ok(Some(m.value().deref().clone()));
        }

        // This takes the write lock!
        match self.cache.entry(key.clone()) {
            Entry::Occupied(entry) => {
                // In case the entry is already initialized, return it. We must not re-initialize
                // it with the base value because there can be a transaction committing this value
                // at the same time and caching it first. Hence, that "newer" value neeSD to stay.
                Ok(Some(entry.get().deref().clone()))
            },
            Entry::Vacant(entry) => match default()? {
                Some(m) => {
                    let m = Arc::new(m);
                    entry.insert(CachePadded::new(m.clone()));
                    Ok(Some(m))
                },
                None => Ok(None),
            },
        }
    }
}
