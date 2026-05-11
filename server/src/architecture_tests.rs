//! Architecture tests — compile-time-style enforcement for codebase rules
//! that don't have a Rust language equivalent. Runs with `cargo test` so CI
//! and local dev both catch violations before they land.

/// Enforce the "`pub` data types live in `models.rs`" rule from CLAUDE.md
/// for `routes.rs` and `middleware.rs` files.
///
/// Implementation files (route handlers, middleware) hold logic; their
/// `pub struct` / `pub enum` / `pub type` definitions belong in a sibling
/// `models.rs`. The rule is **strict by design** — no carve-outs, no
/// "this one is transport-only" judgment calls. Strict means consistent
/// across many small AI-generated contributions.
///
/// **Scope:** currently enforced in `routes.rs` and `middleware.rs` only.
/// Future PRs will extend to `service.rs` (errors), `repo.rs` (DB docs),
/// and other implementation files per CLAUDE.md's strict rule. The check
/// stays narrow until each phase of the cleanup lands so existing inline
/// definitions don't fail this test mid-migration.
#[test]
fn pub_data_types_live_in_models_rs() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src = std::path::Path::new(&manifest_dir).join("src");

    let mut violations: Vec<String> = Vec::new();
    scan(&src, &mut violations);

    if !violations.is_empty() {
        panic!(
            "\nFound {} `pub` data type(s) defined inline in an enforced file:\n\n{}\n\n\
             Per CLAUDE.md Style Guidelines: `pub` data types live in sibling `models.rs` files.\n\
             Move each violation to the matching `models.rs` and import via `use super::models::...;`.\n",
            violations.len(),
            violations.join("\n")
        );
    }
}

/// Enforce the stepdown rule (Clean Code, Robert C. Martin) at the
/// public-vs-private level: in any enforced file, no free private `fn`
/// may appear before the first free public `fn`. The reader sees the
/// file's interface first; helpers come below.
///
/// Caller-above-callee — the deeper half of the rule — needs a real
/// call graph and isn't checked. This catches the dominant "helpers
/// pasted at the top" smell.
///
/// **Free fn = at depth 0.** Methods inside `impl Foo { ... }` and
/// trait method signatures inside `pub trait { ... }` are nested and
/// not checked by this rule.
///
/// **`pub(crate)` counts as public** for this check (unlike the
/// pub-types-in-models rule). The stepdown rule is about reading order
/// within a file; `pub(crate) fn` is the file's visible surface to the
/// rest of the crate, so it should appear above private helpers.
#[test]
fn stepdown_rule_at_depth_zero() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src = std::path::Path::new(&manifest_dir).join("src");

    let mut violations: Vec<String> = Vec::new();
    scan_stepdown(&src, &mut violations);

    if !violations.is_empty() {
        panic!(
            "\nFound {} file(s) where a private `fn` appears before the first public `fn` at depth 0:\n\n{}\n\n\
             Per CLAUDE.md Style Guidelines: source files read top-down — public API first, helpers at the bottom.\n\
             Move the offending helper(s) to a `// ── Helpers ──` section below the public functions, OR\n\
             demote the public function's visibility if it's only used internally.\n",
            violations.len(),
            violations.join("\n")
        );
    }
}

/// Files where the architecture rules must NOT be violated.
///
/// **Denylist**: every `.rs` under `src/` is enforced *except* those
/// listed below. Flipping from an allowlist to a denylist means new
/// helper files (e.g. `landing.rs`, `qr.rs`, `stripe_webhook.rs`)
/// inherit enforcement automatically — the previous allowlist quietly
/// missed them.
///
/// Skipped:
/// - `models.rs` — the destination of the pub-types-in-models rule
/// - `architecture_tests.rs` — this file (don't scan ourselves)
/// - `lib.rs` / `main.rs` — crate roots, contain macro defs and the like
/// - `*_tests.rs` — sibling test files, exempt from style rules
fn is_enforced_file(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    !matches!(
        name,
        "models.rs" | "architecture_tests.rs" | "lib.rs" | "main.rs"
    ) && !name.ends_with("_tests.rs")
}

/// Files with **known pub-data-type violations** that haven't been
/// migrated yet. Flipping `is_enforced_file` to a denylist exposed
/// pre-existing inline `pub` data types in these files. Each entry is
/// debt to clean up — extract the data types to the appropriate
/// `models.rs` and remove the file from this list.
///
/// The stepdown rule and impl_container rule still apply to these
/// files. Only the pub-types-in-models check is suppressed.
///
/// **Do not add new entries.** If you're tempted to add a file here,
/// instead extract the data types it carries to a sibling `models.rs`.
const PUB_TYPES_CLEANUP_BACKLOG: &[&str] = &[
    // TODO: Config (env-loaded settings) — needs core/models.rs or accept as
    // top-level data location.
    "src/core/config.rs",
    // TODO: ClickEventPayload, AttributionEventPayload, ConversionEventPayload,
    // WebhookPayload — extract to a new core/models.rs (these are shared event
    // payload shapes used across services).
    "src/core/webhook_dispatcher.rs",
    // TODO: ErrorResponse, AppError — top-level shared error types. Either
    // create error/models.rs or formally accept top-level error.rs as a
    // models-shaped file.
    "src/error.rs",
    // TODO: rename mcp/tools.rs → mcp/models.rs (the file holds MCP tool input
    // DTOs, fits the models pattern by purpose).
    "src/mcp/tools.rs",
    // TODO: AppState — single-struct top-level file. Either create app/models.rs
    // or formally accept this as a models-shaped file.
    "src/app.rs",
    // TODO: BillingIntent, BillingTier, BillingHandoffError, HandoffOutcome,
    // BillingHandoffConfig — extract to services/billing/models.rs.
    // BillingHandoffService is an impl container (needs impl_container! marker).
    "src/services/billing/handoff.rs",
    // TODO: StripeConfig, StripeError, CheckoutSession, HandoffCheckoutOpts,
    // PortalSession, WebhookVerifyError — extract to services/billing/models.rs.
    "src/services/billing/stripe_client.rs",
    // TODO: PlanLimits → services/billing/models.rs.
    "src/services/billing/limits.rs",
    // TODO: EventCounterDoc → services/billing/models.rs. EventCountersRepo is
    // an impl container (needs impl_container! marker).
    "src/services/billing/repos/event_counters.rs",
    // TODO: StripeDedupDoc → services/billing/models.rs. StripeWebhookDedupRepo
    // is an impl container (needs impl_container! marker).
    "src/services/billing/repos/stripe_webhook_dedup.rs",
];

/// Whether `path` is on the cleanup backlog (suppress pub-types check only).
fn is_cleanup_backlog(path: &std::path::Path) -> bool {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let rel = path.strip_prefix(&manifest_dir).unwrap_or(path);
    let rel_str = rel.to_string_lossy();
    PUB_TYPES_CLEANUP_BACKLOG
        .iter()
        .any(|entry| rel_str == *entry)
}

/// Parse an `impl_container!(Name)` marker line and return the type name.
/// The macro is defined in `crate::lib` and expands to nothing — its sole
/// purpose is to declare that a `pub` type in this file is an implementation
/// container (hosts methods / trait impls) and is exempt from this rule.
///
/// Accepts both forms:
///
/// ```ignore
/// crate::impl_container!(LinksService);
/// impl_container!(LinksService);
/// ```
fn parse_impl_container_marker(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let line = line.strip_prefix("crate::").unwrap_or(line);
    let line = line.strip_prefix("impl_container!(")?;
    let end = line.find(')')?;
    let name = &line[..end];
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
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
        if is_cleanup_backlog(&path) {
            // Tracked debt — see PUB_TYPES_CLEANUP_BACKLOG. Stepdown rule
            // still applies (separate scanner).
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };

        // Pre-pass: collect impl_container!() exemptions declared in this file.
        let exempted: std::collections::HashSet<String> = content
            .lines()
            .filter_map(|l| parse_impl_container_marker(l).map(String::from))
            .collect();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            let Some(name) = parse_pub_type_name(trimmed) else {
                continue;
            };
            if exempted.contains(name) {
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

/// Scan an enforced file's free `fn` declarations (depth 0 only) and
/// record a violation if a private `fn` appears before the first public
/// `fn`. Brace-counting is naive (doesn't handle braces inside string
/// literals or comments); fine for this codebase, edit if a false
/// positive surfaces.
fn scan_stepdown(dir: &std::path::Path, violations: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            scan_stepdown(&path, violations);
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

        let mut depth: i32 = 0;
        let mut first_pub: Option<(usize, String)> = None;
        let mut first_priv: Option<(usize, String)> = None;
        for (i, line) in content.lines().enumerate() {
            if depth == 0 {
                if let Some((vis, name)) = classify_free_fn(line) {
                    let entry = (i + 1, name.to_string());
                    match vis {
                        FnVisibility::Pub => {
                            if first_pub.is_none() {
                                first_pub = Some(entry);
                            }
                        }
                        FnVisibility::Priv => {
                            if first_priv.is_none() {
                                first_priv = Some(entry);
                            }
                        }
                    }
                }
            }
            depth += line.chars().filter(|c| *c == '{').count() as i32;
            depth -= line.chars().filter(|c| *c == '}').count() as i32;
        }

        if let (Some((priv_line, priv_name)), Some((pub_line, pub_name))) =
            (&first_priv, &first_pub)
        {
            if priv_line < pub_line {
                violations.push(format!(
                    "  {}: private fn `{}` at line {} appears before first public fn `{}` at line {}",
                    path.strip_prefix(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                        .unwrap_or(&path)
                        .display(),
                    priv_name,
                    priv_line,
                    pub_name,
                    pub_line,
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FnVisibility {
    Pub,
    Priv,
}

/// Classify a line as a free `fn` declaration and return the visibility plus
/// name. Returns `None` for non-fn lines. Methods inside `impl` blocks aren't
/// at depth 0 — callers gate this with depth tracking. `pub(crate) fn`,
/// `pub(super) fn`, `pub(in path) fn` all count as `Pub` — they're the file's
/// visible surface to other modules.
fn classify_free_fn(line: &str) -> Option<(FnVisibility, &str)> {
    let trimmed = line.trim_start();
    let (is_pub, after_pub) = if let Some(rest) = trimmed.strip_prefix("pub ") {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix("pub(") {
        let close = rest.find(')')?;
        (true, rest[close + 1..].trim_start())
    } else {
        (false, trimmed)
    };
    // Strip leading async/unsafe/const modifiers (any order, repeated).
    let mut rest = after_pub.trim_start();
    loop {
        let next = rest
            .strip_prefix("async ")
            .or_else(|| rest.strip_prefix("unsafe "))
            .or_else(|| rest.strip_prefix("const "))
            .or_else(|| rest.strip_prefix("extern "));
        match next {
            Some(r) => rest = r.trim_start(),
            None => break,
        }
    }
    // Skip extern "C" prefixes after `extern ` consumed above.
    if rest.starts_with('"') {
        let after_open = &rest[1..];
        if let Some(close) = after_open.find('"') {
            rest = after_open[close + 1..].trim_start();
        }
    }
    let after_fn = rest.strip_prefix("fn ")?;
    let name_end = after_fn
        .find(|c: char| c.is_whitespace() || matches!(c, '<' | '(' | ';'))
        .unwrap_or(after_fn.len());
    let name = &after_fn[..name_end];
    if name.is_empty() {
        None
    } else {
        Some((
            if is_pub {
                FnVisibility::Pub
            } else {
                FnVisibility::Priv
            },
            name,
        ))
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

    use super::parse_impl_container_marker;

    #[test]
    fn marker_with_crate_prefix() {
        assert_eq!(
            parse_impl_container_marker("crate::impl_container!(LinksService);"),
            Some("LinksService")
        );
    }

    #[test]
    fn marker_without_crate_prefix() {
        assert_eq!(
            parse_impl_container_marker("impl_container!(LinksRepo);"),
            Some("LinksRepo")
        );
    }

    #[test]
    fn marker_indented() {
        assert_eq!(
            parse_impl_container_marker("    crate::impl_container!(Foo);"),
            Some("Foo")
        );
    }

    #[test]
    fn marker_rejects_non_macro_lines() {
        assert_eq!(
            parse_impl_container_marker("pub struct LinksService {"),
            None
        );
        assert_eq!(parse_impl_container_marker("// just a comment"), None);
        assert_eq!(parse_impl_container_marker("impl_container!();"), None);
    }

    use super::{classify_free_fn, FnVisibility};

    #[test]
    fn fn_classify_public() {
        assert_eq!(
            classify_free_fn("pub fn foo() {"),
            Some((FnVisibility::Pub, "foo"))
        );
        assert_eq!(
            classify_free_fn("pub async fn foo() {"),
            Some((FnVisibility::Pub, "foo"))
        );
        assert_eq!(
            classify_free_fn("pub(crate) fn bar() {"),
            Some((FnVisibility::Pub, "bar"))
        );
        assert_eq!(
            classify_free_fn("pub(super) async fn baz() {"),
            Some((FnVisibility::Pub, "baz"))
        );
    }

    #[test]
    fn fn_classify_private() {
        assert_eq!(
            classify_free_fn("fn helper() {"),
            Some((FnVisibility::Priv, "helper"))
        );
        assert_eq!(
            classify_free_fn("async fn helper() {"),
            Some((FnVisibility::Priv, "helper"))
        );
        assert_eq!(
            classify_free_fn("    fn helper() {"),
            Some((FnVisibility::Priv, "helper"))
        );
    }

    #[test]
    fn fn_classify_rejects_non_fn() {
        assert_eq!(classify_free_fn("pub struct Foo {"), None);
        assert_eq!(classify_free_fn("pub use foo::Bar;"), None);
        assert_eq!(classify_free_fn("pub const X: u32 = 1;"), None);
        assert_eq!(classify_free_fn("// pub fn commented out"), None);
        assert_eq!(classify_free_fn("let x = 1;"), None);
    }

    #[test]
    fn fn_classify_extern() {
        // `extern "C" fn foo()` and `pub extern "C" fn foo()`
        assert_eq!(
            classify_free_fn("extern \"C\" fn foo() {"),
            Some((FnVisibility::Priv, "foo"))
        );
        assert_eq!(
            classify_free_fn("pub extern \"C\" fn foo() {"),
            Some((FnVisibility::Pub, "foo"))
        );
    }
}
