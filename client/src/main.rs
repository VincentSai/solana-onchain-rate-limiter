use solana_onchain_rate_limiter::{Processor, RateLimiterError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn main() {
    println!("============================================================");
    println!(" Solana On-Chain Rate Limiter (Token Bucket) Benchmark CLI ");
    println!("============================================================");

    let admin = [1u8; 32];
    let client_target = [42u8; 32];

    let start_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Policy: Capacity = 10,000 req burst, Refill Rate = 2,500 req/sec
    let capacity = 10_000u64;
    let refill_rate = 2_500u64;
    let mut account = Processor::process_initialize(
        admin,
        client_target,
        capacity,
        refill_rate,
        capacity,
        255,
        start_timestamp,
    )
    .expect("Failed to initialize limiter");

    println!("Initialized Rate Limiter:");
    println!("  Target ID:          {:?}", &client_target[..8]);
    println!("  Bucket Capacity:    {} tokens", account.capacity);
    println!("  Refill Rate:        {} tokens/sec", account.refill_rate_per_sec);
    println!("  Available Tokens:   {}", account.current_tokens);
    println!("------------------------------------------------------------");

    println!("Simulating 50,000 rapid backend microservice requests...");
    let start_bench = Instant::now();
    let mut allowed_count = 0u64;
    let mut throttled_count = 0u64;

    let num_simulated_requests = 50_000;
    for i in 0..num_simulated_requests {
        // Simulating 1 second passing every 2,500 requests
        let simulated_time = start_timestamp + (i / 2_500) as i64;

        match Processor::process_acquire(&mut account, 1, simulated_time) {
            Ok(_) => allowed_count += 1,
            Err(RateLimiterError::RateLimitExceeded { .. }) => throttled_count += 1,
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    let elapsed = start_bench.elapsed();
    let rps = (num_simulated_requests as f64) / elapsed.as_secs_f64();

    println!("Benchmark Complete!");
    println!("  Execution Time:     {:?}", elapsed);
    println!("  Throughput:         {:.2} ops/sec", rps);
    println!("  Allowed Requests:   {}", allowed_count);
    println!("  Throttled Requests: {}", throttled_count);
    println!("  Limiter Total Throttled: {}", account.total_throttled);
    println!("  Limiter Total Requests:  {}", account.total_requests);
    println!("============================================================");
}
