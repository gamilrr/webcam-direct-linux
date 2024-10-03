//! # Error handling utilities.
//! For now we are using generic anyhow error type.
//! Use crate thiserror to reduce boilerplate in custom error types.

pub type Result<T> = anyhow::Result<T>;
