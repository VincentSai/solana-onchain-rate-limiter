use crate::error::{RateLimiterError, Result};
use crate::state::RateLimiterAccount;

pub struct Processor;

impl Processor {
    /// Core lazy refill token bucket logic.
    /// Evaluates the tokens added since `last_refill_timestamp` and updates the available tokens.
    pub fn refill(account: &mut RateLimiterAccount, current_timestamp: i64) -> Result<()> {
        if current_timestamp > account.last_refill_timestamp {
            let elapsed_seconds = (current_timestamp - account.last_refill_timestamp) as u64;

            // Safe multiplication to avoid overflow
            let tokens_to_add = elapsed_seconds
                .checked_mul(account.refill_rate_per_sec)
                .ok_or(RateLimiterError::NumericalOverflow)?;

            // Safe addition capped at capacity
            account.current_tokens = account
                .current_tokens
                .saturating_add(tokens_to_add)
                .min(account.capacity);

            account.last_refill_timestamp = current_timestamp;
        }
        Ok(())
    }

    /// Process Initialize instruction
    pub fn process_initialize(
        admin: [u8; 32],
        target: [u8; 32],
        capacity: u64,
        refill_rate_per_sec: u64,
        initial_tokens: u64,
        bump: u8,
        current_timestamp: i64,
    ) -> Result<RateLimiterAccount> {
        if capacity == 0 {
            return Err(RateLimiterError::InvalidCapacity);
        }
        if refill_rate_per_sec == 0 {
            return Err(RateLimiterError::InvalidRefillRate);
        }

        let tokens = initial_tokens.min(capacity);

        Ok(RateLimiterAccount {
            admin,
            target,
            capacity,
            refill_rate_per_sec,
            last_refill_timestamp: current_timestamp,
            current_tokens: tokens,
            total_requests: 0,
            total_throttled: 0,
            is_initialized: true,
            bump,
        })
    }

    /// Process Acquire tokens instruction
    pub fn process_acquire(
        account: &mut RateLimiterAccount,
        requested_tokens: u64,
        current_timestamp: i64,
    ) -> Result<()> {
        if !account.is_initialized {
            return Err(RateLimiterError::Uninitialized);
        }
        if requested_tokens == 0 {
            return Err(RateLimiterError::InvalidTokenAmount);
        }

        // 1. Refill tokens based on elapsed time
        Self::refill(account, current_timestamp)?;

        // 2. Metric increment
        account.total_requests = account.total_requests.saturating_add(1);

        // 3. Evaluation
        if account.current_tokens >= requested_tokens {
            account.current_tokens -= requested_tokens;
            Ok(())
        } else {
            // Throttled: calculate how many seconds until sufficient tokens are available
            account.total_throttled = account.total_throttled.saturating_add(1);
            let deficit = requested_tokens - account.current_tokens;
            let retry_after = (deficit + account.refill_rate_per_sec - 1) / account.refill_rate_per_sec;
            Err(RateLimiterError::RateLimitExceeded {
                retry_after_seconds: retry_after,
            })
        }
    }

    /// Process UpdatePolicy instruction
    pub fn process_update_policy(
        account: &mut RateLimiterAccount,
        caller: [u8; 32],
        new_capacity: u64,
        new_refill_rate_per_sec: u64,
        current_timestamp: i64,
    ) -> Result<()> {
        if !account.is_initialized {
            return Err(RateLimiterError::Uninitialized);
        }
        if account.admin != caller {
            return Err(RateLimiterError::Unauthorized);
        }
        if new_capacity == 0 {
            return Err(RateLimiterError::InvalidCapacity);
        }
        if new_refill_rate_per_sec == 0 {
            return Err(RateLimiterError::InvalidRefillRate);
        }

        // Settle before updating policy
        Self::refill(account, current_timestamp)?;

        account.capacity = new_capacity;
        account.refill_rate_per_sec = new_refill_rate_per_sec;
        account.current_tokens = account.current_tokens.min(new_capacity);

        Ok(())
    }

    /// Process Reset instruction
    pub fn process_reset(
        account: &mut RateLimiterAccount,
        caller: [u8; 32],
        current_timestamp: i64,
    ) -> Result<()> {
        if !account.is_initialized {
            return Err(RateLimiterError::Uninitialized);
        }
        if account.admin != caller {
            return Err(RateLimiterError::Unauthorized);
        }

        account.current_tokens = account.capacity;
        account.last_refill_timestamp = current_timestamp;
        account.total_throttled = 0;

        Ok(())
    }
}
