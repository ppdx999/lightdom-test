//! Framework-specific transport implementations
//!
//! This module provides HttpTransport implementations for popular web frameworks.

#[cfg(feature = "axum")]
pub mod axum;

#[cfg(feature = "axum")]
pub use self::axum::AxumTransport;

#[cfg(feature = "rocket")]
pub mod rocket;

#[cfg(feature = "rocket")]
pub use self::rocket::RocketTransport;
