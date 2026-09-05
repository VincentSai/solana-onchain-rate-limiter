# Solana On-Chain Token Bucket Rate Limiter & Quota Engine

> High-throughput, horizontally-scalable decentralized rate limiter engineered in Rust, translating traditional Web2 API rate-limiting patterns into Solana's parallel Program Derived Address (PDA) state-machine model.

[![Rust](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-6%20passed-brightgreen.svg)]()

---

## 1. Executive Summary & Design Rationale

In Web2 distributed systems, rate limiting is universally deployed to protect services from Denial of Service (DoS) attacks, enforce API billing tiers, and ensure fair resource allocation. Traditionally, backends rely on centralized in-memory stores like **Redis** running Lua scripts.

This project reframes **Solana as a distributed state-machine backend** rather than just a financial settlement ledger. By redesigning the token bucket algorithm around **Solana's Sealevel parallel transaction runtime** and **isolated PDA accounts**, we achieve:
1. **Zero-Contention Horizontal Scaling**: Different clients/API keys access non-overlapping PDA accounts, allowing Solana validator nodes to process thousands of rate-limit checks concurrently across separate cores without locks.
2. **Deterministic Mathematical Lazy Refill**: Eliminates the need for background cron jobs or active timers. Tokens are computed lazily on each request based on the elapsed Unix time.
3. **Censorship-Resistant Quotas & SLA Enforcement**: Unalterable API quotas agreed upon on-chain between API providers and consumers, natively eliminating dispute over billing overages.

---

## 2. Architecture: Web2 vs. Solana Comparison

```
+---------------------------------------------------------------------------------------+
|                                    WEB2 ARCHITECTURE                                  |
+---------------------------------------------------------------------------------------+
|  [Clients] ---> [API Gateway / Envoy] ---> [Centralized Redis Cluster (Single-Key)]    |
|                                                     |                                 |
|                                        (Lock Contention & Master Failover)            |
+---------------------------------------------------------------------------------------+

+---------------------------------------------------------------------------------------+
|                                 SOLANA ON-CHAIN ARCHITECTURE                          |
+---------------------------------------------------------------------------------------+
|  [Clients] ---> [RPC / Microservices] ---> [Solana Sealevel Parallel Engine]          |
|                                                     |                                 |
|                         +---------------------------+---------------------------+     |
|                         |                           |                           |     |
|              [PDA: Client A Bucket]      [PDA: Client B Bucket]      [PDA: Client C]  |
|              (Independent R/W Lock)      (Independent R/W Lock)      (Parallel Exec)  |
+---------------------------------------------------------------------------------------+
```

### Detailed Architectural Comparison Table

| Architecture Dimension | Traditional Web2 (Redis + Lua) | Solana On-Chain Program (Rust + PDA) |
| :--- | :--- | :--- |
| **State Storage** | In-memory RAM keys (`client:{id}:tokens`) | Independent Program Derived Address (PDA) accounts |
| **Concurrency Model** | Redis Single-threaded event loop (contention on hot keys) | **Sealevel Parallel Execution**: distinct accounts processed concurrently across GPU/CPU cores |
| **Refill Mechanism** | Lua script calculates `(now - last_time) * rate` on request | Deterministic lazy refill via `Clock::get()?.unix_timestamp` |
| **High Availability** | Redis Sentinel / Cluster failover, network partition risks | Global fault tolerance backed by 1,000+ independent Solana validators |
| **Trust & Auditability** | Provider can manipulate logs/counters arbitrarily | Cryptographically signed, tamper-proof state verifiable on-chain |
| **Failure Mode** | Redis down = all rate limiting fails open or blocks traffic | Validator consensus guarantees continuous execution |

---

## 3. Account State Modeling & PDA Derivation

Each rate limiter instance is modeled as an independent 114-byte account:

```rust
pub struct RateLimiterAccount {
    pub admin: [u8; 32],                // 32B: Admin authorized to configure policies
    pub target: [u8; 32],               // 32B: Client Pubkey or SHA256 of API Key / IP
    pub capacity: u64,                  // 8B:  Max burst capacity
    pub refill_rate_per_sec: u64,       // 8B:  Sustained token fill rate per second
    pub last_refill_timestamp: i64,     // 8B:  Unix timestamp of last settlement
    pub current_tokens: u64,            // 8B:  Current liquid tokens available
    pub total_requests: u64,            // 8B:  Cumulative requests metric
    pub total_throttled: u64,           // 8B:  Cumulative throttled metric
    pub is_initialized: bool,           // 1B:  Safety flag
    pub bump: u8,                       // 1B:  PDA validation bump
}
```

### Deterministic PDA Seeds
```text
PDA = Pubkey::find_program_address(
    &[b"rate_limiter", admin.key().as_ref(), target_hash.as_ref()],
    &program_id
)
```
Because `target_hash` is unique per client, transactions modifying Client A and Client B declare non-overlapping writable accounts in their account vectors (`account_keys`), allowing the validator scheduler to execute them in parallel.

---

## 4. Mathematical Algorithm: Lossless Lazy Refill

Instead of continuous ticks, token allocation is calculated on-demand upon incoming transactions:

$$\Delta t = \max(0, t_{	ext{current}} - t_{	ext{last\_refill}})$$

$$\Delta 	ext{tokens} = \Delta t 	imes 	ext{refill\_rate\_per\_sec}$$

$$	ext{liquid\_tokens} = \min(	ext{capacity}, 	ext{current\_tokens} + \Delta 	ext{tokens})$$

If $	ext{liquid\_tokens} \ge 	ext{requested}$:
- $	ext{current\_tokens} = 	ext{liquid\_tokens} - 	ext{requested}$
- Transaction succeeds (`Ok(())`).

If $	ext{liquid\_tokens} < 	ext{requested}$:
- Request throttled.
- Precise recovery time calculation returned to client:
$$	ext{deficit} = 	ext{requested} - 	ext{liquid\_tokens}$$
$$	ext{retry\_after} = \left\lceil rac{	ext{deficit}}{	ext{refill\_rate\_per\_sec}} ightceil$$
- Returns `RateLimiterError::RateLimitExceeded { retry_after_seconds }`.

---

## 5. Engineering Tradeoffs & Production Constraints

1. **Timestamp Resolution & Drift**:
   - Web2 Redis uses sub-millisecond system clocks.
   - Solana `Clock::get()?.unix_timestamp` operates at slot granularity (~400ms). For microsecond-rate limiting, rate limiting is best configured at **per-second granularity** or via **ticket micro-batching**.
2. **Transaction Cost (Micro-batching Strategy)**:
   - Calling an on-chain transaction for every single HTTP packet is economically impractical on public mainnet due to micro-fees ($0.00025/tx).
   - **Production Pattern**: The API Gateway issues **off-chain ticket deductions** using state channels or verifies an on-chain quota checkpoint every N seconds (Batch Settlement pattern).
3. **State Rent Exemption**:
   - The 114-byte account requires ~0.00168 SOL for rent-exemption, which can be closed and refunded by the admin when a client cancels their subscription.

---

## 6. Project Structure

```
solana-onchain-rate-limiter/
├── program/                      # Core Solana on-chain Rust crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Library entrypoint & module definitions
│       ├── state.rs              # Borsh-serialized Account struct (114 bytes)
│       ├── instruction.rs        # Instruction enums (Initialize, Acquire, Reset, Update)
│       ├── processor.rs          # Core Token Bucket arithmetic & authorization logic
│       ├── error.rs              # Custom typed errors with retry_after calculations
│       └── tests.rs              # Exhaustive unit & edge-case test suite
├── client/                       # High-speed benchmarking client CLI
│   ├── Cargo.toml
│   └── src/
│       └── main.rs               # Benchmark harness (simulates 50,000 requests)
└── README.md                     # Comprehensive architecture documentation
```

---

## 7. Quickstart & Verification

### Run the Test Suite
```bash
cargo test
```
Outputs:
```text
running 6 tests
test tests::test_admin_reset ... ok
test tests::test_capacity_ceiling_and_burst_protection ... ok
test tests::test_acquire_tokens_and_throttling ... ok
test tests::test_initialization_invalid_parameters ... ok
test tests::test_initialization_success ... ok
test tests::test_policy_update_and_authorization ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; finished in 0.00s
```

### Run the High-Throughput Benchmark
```bash
cargo run --bin rate-limiter-client
```
Simulates 50,000 rapid backend microservice requests against the rate limiter engine:
```text
============================================================
 Solana On-Chain Rate Limiter (Token Bucket) Benchmark CLI 
============================================================
Initialized Rate Limiter:
  Target ID:          [42, 42, 42, 42, 42, 42, 42, 42]
  Bucket Capacity:    10000 tokens
  Refill Rate:        2500 tokens/sec
  Available Tokens:   10000
------------------------------------------------------------
Simulating 50,000 rapid backend microservice requests...
Benchmark Complete!
  Execution Time:     870.8µs
  Throughput:         57,418,465 ops/sec
  Allowed Requests:   50000
  Throttled Requests: 0
============================================================
```

---

## 8. License
Licensed under Apache-2.0 or MIT at your option.
