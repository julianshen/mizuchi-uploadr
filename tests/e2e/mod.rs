//! End-to-End Tests for Mizuchi Uploadr
//!
//! This module contains comprehensive E2E tests that validate the complete
//! upload proxy system including:
//!
//! - Full upload flow (simple PUT + multipart)
//! - Authentication integration (JWT + SigV4)
//! - Authorization enforcement (OPA + OpenFGA)
//! - Error handling scenarios
//! - Performance and load testing
//!
//! ## Test Infrastructure
//!
//! Tests require a running S3-compatible backend (RustFS recommended).
//! Use docker-compose to start the test environment:
//!
//! ```bash
//! docker-compose -f docker-compose.e2e.yml up -d
//! ```
//!
//! ## Test Categories
//!
//! - `upload_flow`: Happy path upload tests
//! - `auth_flow`: Authentication + upload integration
//! - `error_scenarios`: Error handling validation
//! - `load_test`: Performance and load testing

pub mod auth_flow;
pub mod common;
pub mod error_scenarios;
pub mod load_test;
pub mod upload_flow;
