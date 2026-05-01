#[cfg(feature = "api")]
pub mod api;
pub mod app;
pub mod core;
pub mod error;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod services;

/// Marker macro: declares that the named `pub` type is an implementation
/// container (a type whose role is to host its `impl` block), exempt from
/// the "pub data types live in models.rs" architecture rule.
///
/// Usage — place above the type definition (and above any `#[derive]` /
/// doc-comment block) in any enforced implementation file:
///
/// ```ignore
/// crate::impl_container!(LinksService);
/// pub struct LinksService { /* ... */ }
/// ```
///
/// Expands to nothing. `src/architecture_tests.rs` scans for invocations
/// and exempts the named type from the data-types-in-models check.
///
/// Use this macro instead of an ad-hoc naming convention so the exemption
/// is explicit, grep-able, and decoupled from how the type is named.
#[macro_export]
macro_rules! impl_container {
    ($name:ident) => {};
}

#[cfg(test)]
mod architecture_tests;
