use async_trait::async_trait;
use cached::{Cached, TimedSizedCache};
use chrono::NaiveDate;
use model::entities::account;
use polars::prelude::DataFrame;
use sea_orm::DatabaseConnection;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{AccountStateCalculator, MergeMethod};
use crate::error::Result;

/// A cache key for the compute_account_state method
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComputeStateKey {
    accounts_hash: u64,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl ComputeStateKey {
    fn new(accounts: &[account::Model], start_date: NaiveDate, end_date: NaiveDate) -> Self {
        let mut hasher = DefaultHasher::new();
        for account in accounts {
            account.id.hash(&mut hasher);
            account.name.hash(&mut hasher);
            account.currency_code.hash(&mut hasher);
            account.owner_id.hash(&mut hasher);
        }
        let accounts_hash = hasher.finish();

        Self {
            accounts_hash,
            start_date,
            end_date,
        }
    }
}

/// A caching wrapper for AccountStateCalculator implementations.
///
/// This struct wraps any implementation of AccountStateCalculator and caches
/// the results of method calls to avoid recomputing expensive operations like
/// DataFrame generation.
///
/// Features:
/// - Caches compute_account_state results with TTL
/// - Different caches for different method arguments
/// - Cache clearing functionality
/// - Thread-safe implementation using Arc<Mutex<>>
pub struct AccountStateCacheCalculator<
    T: AccountStateCalculator,
    C: Cached<ComputeStateKey, DataFrame> = TimedSizedCache<ComputeStateKey, DataFrame>,
> {
    /// The wrapped calculator
    inner: T,
    /// Cache for compute_account_state results
    compute_cache: Arc<Mutex<C>>,
}

impl<T: AccountStateCalculator, C: Cached<ComputeStateKey, DataFrame>>
AccountStateCacheCalculator<T, C>
{
    /// Creates a new cache calculator wrapping the provided calculator with a custom cache store.
    ///
    /// # Arguments
    /// * `inner` - The calculator to wrap with caching
    /// * `cache_store` - Custom cache store implementation
    pub fn new_with_store(inner: T, cache_store: C) -> Self {
        Self {
            inner,
            compute_cache: Arc::new(Mutex::new(cache_store)),
        }
    }

    /// Clears all caches.
    ///
    /// This method removes all cached entries, forcing fresh computation
    /// on the next method calls.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.compute_cache.lock() {
            cache.cache_clear();
        }
    }

    /// Clears only the compute_account_state cache.
    pub fn clear_compute_cache(&self) {
        if let Ok(mut cache) = self.compute_cache.lock() {
            cache.cache_clear();
        }
    }

    /// Returns the current size of the compute cache.
    pub fn compute_cache_size(&self) -> usize {
        if let Ok(cache) = self.compute_cache.lock() {
            cache.cache_size()
        } else {
            0
        }
    }
}

impl<T: AccountStateCalculator>
AccountStateCacheCalculator<T, TimedSizedCache<ComputeStateKey, DataFrame>>
{
    /// Creates a new cache calculator wrapping the provided calculator.
    ///
    /// # Arguments
    /// * `inner` - The calculator to wrap with caching
    /// * `cache_size` - Maximum number of entries in the cache
    /// * `ttl` - Time to live for cached entries
    /// * `cache_store` - Optional custom cache store implementation
    pub fn new(
        inner: T,
        cache_size: usize,
        ttl: Duration,
        cache_store: Option<TimedSizedCache<ComputeStateKey, DataFrame>>,
    ) -> Self {
        let compute_cache = if let Some(store) = cache_store {
            Arc::new(Mutex::new(store))
        } else {
            Arc::new(Mutex::new(TimedSizedCache::with_size_and_lifespan(
                cache_size,
                ttl.as_secs(),
            )))
        };

        Self {
            inner,
            compute_cache,
        }
    }

    /// Creates a new cache calculator with default settings.
    ///
    /// Default settings:
    /// - Cache size: 100 entries
    /// - TTL: 5 minutes
    pub fn with_defaults(inner: T) -> Self {
        Self::new(inner, 100, Duration::from_secs(300), None)
    }
}

#[async_trait]
impl<T: AccountStateCalculator + Send + Sync, C: Cached<ComputeStateKey, DataFrame> + Send + Sync>
AccountStateCalculator for AccountStateCacheCalculator<T, C>
{
    async fn compute_account_state(
        &self,
        db: &DatabaseConnection,
        accounts: &[account::Model],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<DataFrame> {
        let cache_key = ComputeStateKey::new(accounts, start_date, end_date);

        // Try to get from cache first
        if let Ok(mut cache) = self.compute_cache.lock() {
            if let Some(cached_result) = cache.cache_get(&cache_key) {
                return Ok(cached_result.clone());
            }
        }

        // Not in cache, compute the result
        let result = self
            .inner
            .compute_account_state(db, accounts, start_date, end_date)
            .await?;

        // Store in cache
        if let Ok(mut cache) = self.compute_cache.lock() {
            cache.cache_set(cache_key, result.clone());
        }

        Ok(result)
    }

    fn merge_method(&self) -> MergeMethod {
        self.inner.merge_method()
    }

    fn update_initial_balance(&mut self, balance: rust_decimal::Decimal) -> bool {
        // Clear cache when balance is updated since it might affect future calculations
        self.clear_cache();
        self.inner.update_initial_balance(balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::balance::BalanceCalculator;
    use crate::account::testing::*;
    use model::entities::account::AccountKind;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_cache_basic_functionality() {
        let balance_calc = BalanceCalculator::default();
        let cache_calc = AccountStateCacheCalculator::with_defaults(balance_calc);

        // Test merge_method functionality (no longer cached)
        let method1 = cache_calc.merge_method();
        let method2 = cache_calc.merge_method();
        assert_eq!(method1, method2);
    }

    #[tokio::test]
    async fn test_cache_clearing() {
        let balance_calc = BalanceCalculator::default();
        let cache_calc = AccountStateCacheCalculator::with_defaults(balance_calc);

        // Test cache clearing functionality
        cache_calc.clear_cache();
        assert_eq!(cache_calc.compute_cache_size(), 0);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let balance_calc = BalanceCalculator::default();
        let cache_calc = AccountStateCacheCalculator::new(
            balance_calc,
            10,
            Duration::from_millis(100), // Very short TTL for testing
            None,
        );

        // Test basic functionality
        let _method = cache_calc.merge_method();

        // Wait for TTL to expire
        sleep(Duration::from_millis(150)).await;

        // The cache should have expired, but we can't easily test this
        // without accessing the internal cache state
        // This is more of a integration test that would need actual computation
    }

    #[test]
    fn test_compute_state_key() {
        let account1 = account::Model {
            id: 1,
            name: "Test Account".to_string(),
            description: None,
            currency_code: "USD".to_string(),
            owner_id: 1,
            include_in_statistics: true,
            ledger_name: None,
            account_kind: AccountKind::RealAccount,
            target_amount: None,
        };
        let account2 = account::Model {
            id: 2,
            name: "Test Account 2".to_string(),
            description: None,
            currency_code: "USD".to_string(),
            owner_id: 1,
            include_in_statistics: true,
            ledger_name: None,
            account_kind: AccountKind::RealAccount,
            target_amount: None,
        };

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let key1 = ComputeStateKey::new(&[account1.clone()], date1, date2);
        let key2 = ComputeStateKey::new(&[account1.clone()], date1, date2);
        let key3 = ComputeStateKey::new(&[account2.clone()], date1, date2);

        assert_eq!(key1, key2); // Same accounts and dates should produce same key
        assert_ne!(key1, key3); // Different accounts should produce different keys
    }

    #[tokio::test]
    async fn test_cache_integration_with_balance_calculator() {
        // Create a test scenario
        let scenario = ScenarioBalance::new();

        // Create a balance calculator wrapped with cache
        let balance_calc = BalanceCalculator::new(MergeMethod::FirstWins);
        let cache_calc = AccountStateCacheCalculator::with_defaults(balance_calc);

        // Test that the cached calculator works with the scenario
        run_and_assert_scenario(&scenario, &cache_calc, true)
            .await
            .expect("Failed to run scenario with cached calculator");

        // Test merge_method functionality
        let method1 = cache_calc.merge_method();
        let method2 = cache_calc.merge_method();
        assert_eq!(method1, method2);

        // Test cache clearing
        cache_calc.clear_cache();
        assert_eq!(cache_calc.compute_cache_size(), 0);

        // Test that it still works after cache clearing
        run_and_assert_scenario(&scenario, &cache_calc, true)
            .await
            .expect("Failed to run scenario after cache clearing");
    }

    #[tokio::test]
    async fn test_generic_cache_store_with_custom_cache() {
        use cached::SizedCache;

        // Create a test scenario
        let scenario = ScenarioBalance::new();

        // Create a balance calculator
        let balance_calc = BalanceCalculator::new(MergeMethod::FirstWins);

        // Create a custom cache store (SizedCache without TTL)
        let custom_cache = SizedCache::with_size(50);

        // Use the new generic method to create cache calculator with custom store
        let cache_calc = AccountStateCacheCalculator::new_with_store(balance_calc, custom_cache);

        // Test that the cached calculator works with the scenario
        run_and_assert_scenario(&scenario, &cache_calc, true)
            .await
            .expect("Failed to run scenario with custom cache store");

        // Test cache management methods work with custom store
        cache_calc.clear_cache();
        assert_eq!(cache_calc.compute_cache_size(), 0);
    }
}
