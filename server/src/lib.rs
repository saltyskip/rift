#[cfg(feature = "api")]
pub mod api;
pub mod app;
pub mod core;
pub mod error;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod services;

#[cfg(test)]
mod architecture_tests;
