//! Architecture tests — compile-time-style enforcement for codebase rules
//! that don't have a Rust language equivalent. Runs with `cargo test` so CI
//! and local dev both catch violations before they land.

/// Enforce the "`pub` data types live in `models.rs`" rule from CLAUDE.md
/// for `routes.rs` and `middleware.rs` files.
///
/// Implementation files (route handlers, middleware) hold logic; their
/// `pub struct` / `pub enum` definitions belong in a sibling `models.rs`.
/// This test scans `src/api/` and `src/services/` for offenders and fails
/// with a list of violations + the rule.
///
/// **Carve-outs (allowed inline):**
/// - `*Query` types — pure URL-param decoders, no domain meaning
/// - Names listed in `ALLOWED_INLINE_NAMES` below
///
/// **Scope:** currently enforced in `routes.rs` and `middleware.rs` only.
/// Future PRs will extend to `service.rs` (errors) and `repo.rs` (DB docs)
/// per CLAUDE.md's strict rule. The check stays narrow until each phase
/// of the cleanup lands so existing inline definitions don't fail this
/// test mid-migration.
#[test]
fn pub_data_types_live_in_models_rs() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src = std::path::Path::new(&manifest_dir).join("src");

    let mut violations: Vec<String> = Vec::new();
    scan(&src, &mut violations);

    if !violations.is_empty() {
        panic!(
            "\nFound {} `pub` data type(s) defined inline in `routes.rs` or `middleware.rs`:\n\n{}\n\n\
             Per CLAUDE.md Style Guidelines: `pub` data types live in sibling `models.rs` files.\n\
             Move each violation to the matching `models.rs` and import via `use super::models::...;`.\n\n\
             If the type is a transport-only URL-param decoder, name it `*Query` to match the carve-out,\n\
             or add it to ALLOWED_INLINE_NAMES in this test with a justification.\n",
            violations.len(),
            violations.join("\n")
        );
    }
}

/// Files where `pub` types must NOT appear (must move to sibling `models.rs`).
fn is_enforced_file(path: &std::path::Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("routes.rs") | Some("middleware.rs")
    )
}

/// Specific names that are allowed to remain inline despite being `pub`.
/// Keep this list short and add a comment for each entry.
const ALLOWED_INLINE_NAMES: &[&str] = &[
    // (none currently — `*Query` carve-out below covers all current cases)
];

/// `*Query` types are pure axum `Query<T>` URL-param decoders — transport
/// plumbing with no domain meaning. They have no consumer outside the HTTP
/// transport layer, so the "would this exist in MCP?" test fails for them.
fn is_carved_out(name: &str) -> bool {
    name.ends_with("Query") || ALLOWED_INLINE_NAMES.contains(&name)
}

fn scan(dir: &std::path::Path, violations: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            scan(&path, violations);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        if !is_enforced_file(&path) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            let Some(name) = parse_pub_type_name(trimmed) else {
                continue;
            };
            if is_carved_out(name) {
                continue;
            }
            violations.push(format!(
                "  {}:{} — pub type `{}`",
                path.strip_prefix(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                    .unwrap_or(&path)
                    .display(),
                line_num + 1,
                name,
            ));
        }
    }
}

/// Match `pub struct Foo`, `pub enum Foo`, or `pub type Foo = ...` and
/// return the type name. Returns `None` for non-matches (including
/// `pub(crate)` / `pub(super)` since those don't escape the crate API).
fn parse_pub_type_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("pub ")?;
    // Reject pub(crate), pub(super), pub(in path) — these don't escape.
    if rest.starts_with('(') {
        return None;
    }
    let rest = rest
        .strip_prefix("struct ")
        .or_else(|| rest.strip_prefix("enum "))
        .or_else(|| rest.strip_prefix("type "))?;
    // Pull out the identifier — stops at whitespace, `<`, `(`, `;`, `{`, or `=`.
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '<' | '(' | ';' | '{' | '='))
        .unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

#[cfg(test)]
mod parser_tests {
    use super::parse_pub_type_name;

    #[test]
    fn matches_pub_struct() {
        assert_eq!(parse_pub_type_name("pub struct Foo {"), Some("Foo"));
        assert_eq!(parse_pub_type_name("pub struct Foo;"), Some("Foo"));
        assert_eq!(parse_pub_type_name("pub struct Foo<'a, T> {"), Some("Foo"));
    }

    #[test]
    fn matches_pub_enum() {
        assert_eq!(parse_pub_type_name("pub enum Bar {"), Some("Bar"));
    }

    #[test]
    fn matches_pub_type_alias() {
        assert_eq!(parse_pub_type_name("pub type Baz = String;"), Some("Baz"));
    }

    #[test]
    fn rejects_pub_crate() {
        assert_eq!(parse_pub_type_name("pub(crate) struct Foo {"), None);
        assert_eq!(parse_pub_type_name("pub(super) enum Bar {"), None);
    }

    #[test]
    fn rejects_non_type_pub() {
        assert_eq!(parse_pub_type_name("pub fn foo() {"), None);
        assert_eq!(parse_pub_type_name("pub const X: u32 = 1;"), None);
        assert_eq!(parse_pub_type_name("pub use foo::Bar;"), None);
    }
}
