use super::*;
use crate::error::RateLimiterError;

#[test]
fn test_initialization_success() {
    let admin = [1u8; 32];
    let target = [2u8; 32];
    let now = 1000;

    let account = Processor::process_initialize(admin, target, 50, 10, 50, 255, now).unwrap();

    assert_eq!(account.capacity, 50);
    assert_eq!(account.refill_rate_per_sec, 10);
    assert_eq!(account.current_tokens, 50);
    assert_eq!(account.last_refill_timestamp, 1000);
    assert!(account.is_initialized);
    assert_eq!(account.total_requests, 0);
    assert_eq!(account.total_throttled, 0);
}

#[test]
fn test_initialization_invalid_parameters() {
    let admin = [1u8; 32];
    let target = [2u8; 32];

    assert_eq!(
        Processor::process_initialize(admin, target, 0, 10, 50, 255, 1000).unwrap_err(),
        RateLimiterError::InvalidCapacity
    );

    assert_eq!(
        Processor::process_initialize(admin, target, 50, 0, 50, 255, 1000).unwrap_err(),
        RateLimiterError::InvalidRefillRate
    );
}

#[test]
fn test_acquire_tokens_and_throttling() {
    let admin = [1u8; 32];
    let target = [2u8; 32];
    let now = 1000;

    // Capacity 20, refill rate 5 tokens/sec, initial 20 tokens
    let mut account = Processor::process_initialize(admin, target, 20, 5, 20, 255, now).unwrap();

    // 1. Consume 15 tokens
    Processor::process_acquire(&mut account, 15, now).unwrap();
    assert_eq!(account.current_tokens, 5);
    assert_eq!(account.total_requests, 1);
    assert_eq!(account.total_throttled, 0);

    // 2. Try to consume 10 tokens (only 5 available -> should throttle)
    let err = Processor::process_acquire(&mut account, 10, now).unwrap_err();
    assert_eq!(account.total_requests, 2);
    assert_eq!(account.total_throttled, 1);
    assert_eq!(account.current_tokens, 5); // Tokens unchanged

    // Deficit = 10 - 5 = 5. Rate = 5/sec -> retry_after = ceil(5/5) = 1 sec
    assert_eq!(
        err,
        RateLimiterError::RateLimitExceeded {
            retry_after_seconds: 1
        }
    );

    // 3. Advance time by 2 seconds -> should add 2 * 5 = 10 tokens -> total 15 tokens
    let now_plus_2 = now + 2;
    Processor::process_acquire(&mut account, 10, now_plus_2).unwrap();
    assert_eq!(account.current_tokens, 5); // (5 + 10) - 10 = 5
    assert_eq!(account.total_requests, 3);
}

#[test]
fn test_capacity_ceiling_and_burst_protection() {
    let admin = [1u8; 32];
    let target = [2u8; 32];
    let now = 1000;

    let mut account = Processor::process_initialize(admin, target, 30, 10, 10, 255, now).unwrap();

    // Advance 100 seconds (theoretically generates 1,000 tokens, but capped at capacity 30)
    Processor::refill(&mut account, now + 100).unwrap();
    assert_eq!(account.current_tokens, 30);
    assert_eq!(account.last_refill_timestamp, now + 100);
}

#[test]
fn test_policy_update_and_authorization() {
    let admin = [1u8; 32];
    let target = [2u8; 32];
    let impostor = [9u8; 32];
    let now = 1000;

    let mut account = Processor::process_initialize(admin, target, 50, 5, 50, 255, now).unwrap();

    // Impostor attempts to modify policy
    assert_eq!(
        Processor::process_update_policy(&mut account, impostor, 100, 20, now).unwrap_err(),
        RateLimiterError::Unauthorized
    );

    // Admin updates policy
    Processor::process_update_policy(&mut account, admin, 100, 20, now).unwrap();
    assert_eq!(account.capacity, 100);
    assert_eq!(account.refill_rate_per_sec, 20);
}

#[test]
fn test_admin_reset() {
    let admin = [1u8; 32];
    let target = [2u8; 32];
    let now = 1000;

    let mut account = Processor::process_initialize(admin, target, 50, 5, 50, 255, now).unwrap();

    // Deplete all tokens and record throttles
    Processor::process_acquire(&mut account, 50, now).unwrap();
    assert_eq!(account.current_tokens, 0);

    let _ = Processor::process_acquire(&mut account, 10, now);
    assert_eq!(account.total_throttled, 1);

    // Reset
    Processor::process_reset(&mut account, admin, now).unwrap();
    assert_eq!(account.current_tokens, 50);
    assert_eq!(account.total_throttled, 0);
}
