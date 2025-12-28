//! E2E Test Suite Entry Point
//!
//! This is the main entry point for running E2E tests.
//!
//! ## Prerequisites
//!
//! 1. Start the test infrastructure:
//!    ```bash
//!    docker-compose -f docker-compose.e2e.yml up -d
//!    ```
//!
//! 2. Wait for RustFS to be healthy:
//!    ```bash
//!    docker-compose -f docker-compose.e2e.yml ps
//!    ```
//!
//! 3. Run the tests:
//!    ```bash
//!    cargo test --test e2e_test
//!    ```
//!
//! 4. Cleanup:
//!    ```bash
//!    docker-compose -f docker-compose.e2e.yml down -v
//!    ```
//!
//! ## Test Categories
//!
//! - `upload_flow`: Happy path upload tests (PUT, multipart)
//! - `auth_flow`: Authentication integration tests (JWT, SigV4)
//! - `error_scenarios`: Error handling validation
//! - `load_test`: Performance and load testing

mod e2e;

// Re-export all E2E tests
pub use e2e::*;
