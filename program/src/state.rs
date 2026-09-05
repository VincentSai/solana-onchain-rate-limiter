use borsh::{BorshDeserialize, BorshSerialize};

/// On-chain Rate Limiter Account State.
/// Stored in a Program Derived Address (PDA) deterministically derived from:
/// `[b"rate_limiter", admin_pubkey, target_identifier]`
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RateLimiterAccount {
    /// Administrator / Owner public key who has rights to update policy or reset
    pub admin: [u8; 32],

    /// Target identifier (Client Pubkey, API Key SHA256, or IP Hash)
    pub target: [u8; 32],

    /// Maximum bucket token capacity (burst limit)
    pub capacity: u64,

    /// Number of tokens refilled per second
    pub refill_rate_per_sec: u64,

    /// Unix timestamp (in seconds) of the last lazy token refill
    pub last_refill_timestamp: i64,

    /// Current available tokens in the bucket (fixed-point precision not needed for integer tokens)
    pub current_tokens: u64,

    /// Metric: Total requests processed through this limiter
    pub total_requests: u64,

    /// Metric: Total requests throttled / rejected due to rate limits
    pub total_throttled: u64,

    /// Whether this account has been initialized
    pub is_initialized: bool,

    /// PDA bump seed for address verification
    pub bump: u8,
}

impl RateLimiterAccount {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 1; // 114 bytes
}
