use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum RateLimiterInstruction {
    /// Initializes a new rate limiter bucket for a target.
    ///
    /// Accounts expected:
    /// 0. `[signer]` Admin account
    /// 1. `[writable]` Rate limiter PDA account
    /// 2. `[]` Clock sysvar (optional in pure mock, required on-chain)
    Initialize {
        capacity: u64,
        refill_rate_per_sec: u64,
        initial_tokens: u64,
        target: [u8; 32],
        bump: u8,
    },

    /// Acquires a specified amount of tokens from the bucket.
    ///
    /// Accounts expected:
    /// 0. `[signer]` Caller account (target or authorized client)
    /// 1. `[writable]` Rate limiter PDA account
    /// 2. `[]` Clock sysvar
    Acquire {
        requested_tokens: u64,
    },

    /// Updates the rate limiting policy (capacity and refill rate).
    ///
    /// Accounts expected:
    /// 0. `[signer]` Admin account
    /// 1. `[writable]` Rate limiter PDA account
    UpdatePolicy {
        new_capacity: u64,
        new_refill_rate_per_sec: u64,
    },

    /// Resets the bucket to full capacity and resets throttle counters.
    ///
    /// Accounts expected:
    /// 0. `[signer]` Admin account
    /// 1. `[writable]` Rate limiter PDA account
    Reset,
}
