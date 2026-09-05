use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, PartialEq, Eq)]
pub enum RateLimiterError {
    #[error("Rate limit exceeded. Please retry after the specified seconds.")]
    RateLimitExceeded { retry_after_seconds: u64 },

    #[error("The operation is unauthorized.")]
    Unauthorized,

    #[error("The account is already initialized.")]
    AlreadyInitialized,

    #[error("The account is uninitialized.")]
    Uninitialized,

    #[error("Invalid token amount requested. Must be greater than 0.")]
    InvalidTokenAmount,

    #[error("Numerical overflow occurred during rate limit calculation.")]
    NumericalOverflow,

    #[error("Invalid refill rate. Refill rate must be greater than 0.")]
    InvalidRefillRate,

    #[error("Invalid capacity. Capacity must be greater than 0.")]
    InvalidCapacity,
}

pub type Result<T> = std::result::Result<T, RateLimiterError>;
