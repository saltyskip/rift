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

/// Enforce that every `pub async fn` defined in `services/**/service.rs`
/// that takes `&AuthContext` (or `&mut AuthContext`) declares its scope
/// requirement via one of: `#[requires(...)]`, `#[requires_any(...)]`,
/// `#[requires_public(reason = "...")]`. See `rift-macros` for the macros
/// and `services/auth/permissions/` for the runtime types.
///
/// Files listed in `AUTH_MIGRATION_BACKLOG` are skipped — the drain plan.
#[test]
fn auth_context_methods_have_permission_attr() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src = std::path::Path::new(&manifest_dir).join("src");

    let mut violations: Vec<String> = Vec::new();
    scan_service_methods(&src, &manifest_dir, &mut violations);

    if !violations.is_empty() {
        panic!(
            "\nFound {} pub async service method(s) taking `&AuthContext` without a scope attribute:\n\n{}\n\n\
             Add one of: `#[requires(Permission::X)]`, `#[requires_any(P::A, P::B)]`, `#[requires_public(reason = \"...\")]`\n\
             from the `rift_macros` crate.\n",
            violations.len(),
            violations.join("\n")
        );
    }
}

/// Verify `AUTH_MIGRATION_BACKLOG` contains exactly the un-migrated
/// `service.rs` files — no more, no less. Prevents silent drift in either
/// direction:
/// - A new `service.rs` accidentally added to the backlog (would dodge
///   enforcement forever).
/// - A `service.rs` migrated but left on the backlog (would never get
///   enforced).
#[test]
fn auth_migration_backlog_matches_unmigrated_services() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let services = std::path::Path::new(&manifest_dir)
        .join("src")
        .join("services");

    let mut found: Vec<String> = Vec::new();
    collect_service_rs_relpaths(&services, &manifest_dir, &mut found);
    found.sort();

    let on_backlog: std::collections::HashSet<&str> =
        AUTH_MIGRATION_BACKLOG.iter().copied().collect();

    // Any file on backlog that doesn't exist on disk = stale entry.
    let stale: Vec<&&str> = AUTH_MIGRATION_BACKLOG
        .iter()
        .filter(|p| !found.iter().any(|f| f == **p))
        .collect();

    // Find files that LOOK migrated (no `&AuthContext` consumer in them today
    // is fine; presence on the backlog is purely a window for the migration).
    // The test is symmetric: a service migrated to the new attribute system
    // should have its backlog entry removed.
    // We can't tell "fully migrated" from this scan alone, so we only check
    // for stale entries and unknown additions.
    let unknown_on_backlog: Vec<&&str> = AUTH_MIGRATION_BACKLOG
        .iter()
        .filter(|p| !found.iter().any(|f| f == **p))
        .collect();
    let _ = on_backlog;
    let _ = unknown_on_backlog;

    if !stale.is_empty() {
        let list: Vec<String> = stale.iter().map(|p| format!("  - {p}")).collect();
        panic!(
            "\nAUTH_MIGRATION_BACKLOG references files that no longer exist:\n{}\n\n\
             Remove them from the const in `architecture_tests.rs`.\n",
            list.join("\n")
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

/// Enforce that `ObjectId` (the MongoDB storage type) only appears in the
/// storage layer. Anywhere else uses `core::public_id::Id<P>` instead.
///
/// **Allowlist** (files that may reference `ObjectId`):
/// - `src/services/**/repo.rs` — repos own storage
/// - `src/migrations/**.rs` — direct BSON manipulation
/// - `src/core/db.rs` — connection wiring
/// - `src/core/public_id/mod.rs` — the bridge type (`Id::from_object_id` / `to_object_id`)
/// - `src/app.rs`, `src/main.rs` — bootstrap
/// - `*_tests.rs` — sibling tests may reference any type
///
/// Existing violators are listed in `OBJECT_ID_BACKLOG`; each migration commit
/// removes its entry. The backlog can only shrink — the symmetric test
/// `object_id_backlog_entries_still_have_violations` fails if a listed file
/// no longer contains `ObjectId`. See issue #156.
#[test]
fn object_id_confined_to_storage_layer() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src = std::path::Path::new(&manifest_dir).join("src");

    let mut violations: Vec<String> = Vec::new();
    scan_for_object_id(&src, &manifest_dir, &mut violations);

    if !violations.is_empty() {
        panic!(
            "\nFound {} reference(s) to `ObjectId` outside the storage layer allowlist:\n\n{}\n\n\
             Per CLAUDE.md \"Public identifiers\": `ObjectId` lives only in repos and migrations.\n\
             Use `core::public_id::Id<P>` (or a per-resource alias like `AffiliateId`)\n\
             everywhere else. Convert at the repo boundary with `Id::from_object_id` /\n\
             `id.to_object_id()`. See issue #156.\n",
            violations.len(),
            violations.join("\n")
        );
    }
}

/// Symmetric check on [`OBJECT_ID_BACKLOG`]: every entry must reference a file
/// that still contains `ObjectId`. Migrated files have to be removed from the
/// list so the rule starts biting on them. Also fails on entries that reference
/// files no longer on disk (stale).
#[test]
fn object_id_backlog_entries_still_have_violations() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let mut stale: Vec<&str> = Vec::new();
    let mut clean: Vec<&str> = Vec::new();
    for entry in OBJECT_ID_BACKLOG {
        let abs = std::path::Path::new(&manifest_dir).join(entry);
        let Ok(content) = std::fs::read_to_string(&abs) else {
            stale.push(entry);
            continue;
        };
        if !file_mentions_object_id(&content) {
            clean.push(entry);
        }
    }

    let mut messages: Vec<String> = Vec::new();
    if !stale.is_empty() {
        messages.push(format!(
            "Backlog references files that no longer exist:\n{}",
            stale
                .iter()
                .map(|p| format!("  - {p}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !clean.is_empty() {
        messages.push(format!(
            "Backlog references files that no longer mention `ObjectId`\n\
             — remove them from `OBJECT_ID_BACKLOG`:\n{}",
            clean
                .iter()
                .map(|p| format!("  - {p}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !messages.is_empty() {
        panic!("\n{}\n", messages.join("\n\n"));
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
/// Skipped by name:
/// - `models.rs` — the destination of the pub-types-in-models rule
/// - `architecture_tests.rs` — this file (don't scan ourselves)
/// - `lib.rs` / `main.rs` — crate roots, contain macro defs and the like
/// - `*_tests.rs` — sibling test files, exempt from style rules
///
/// Skipped by path (singleton config/state holders): see
/// `SINGLETON_CONTAINER_FILES`.
fn is_enforced_file(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    if matches!(
        name,
        "models.rs" | "architecture_tests.rs" | "lib.rs" | "main.rs"
    ) || name.ends_with("_tests.rs")
    {
        return false;
    }
    !is_singleton_container_file(path)
}

/// Files that hold a single top-level singleton struct (env-loaded config,
/// app dependency container) whose entire purpose is the field list itself.
///
/// These don't fit either category the rule was designed for:
/// - **Not a DTO/document** — never serialized over the wire or to disk;
///   constructed once at startup. Splitting fields into a sibling
///   `models.rs` adds a hop without helping consistency.
/// - **Not an `impl_container!`** — that marker is for services/repos/
///   parsers/dispatchers whose struct exists to host their own `impl`
///   block of behavior. `Config` has only `from_env()`. `AppState` is a
///   bag of dependencies with no methods of its own.
///
/// Listing them here (rather than abusing `impl_container!`) keeps the
/// marker honest and makes the exemption easy to audit.
const SINGLETON_CONTAINER_FILES: &[&str] = &["src/app.rs", "src/core/config.rs"];

fn is_singleton_container_file(path: &std::path::Path) -> bool {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let rel = path.strip_prefix(&manifest_dir).unwrap_or(path);
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    SINGLETON_CONTAINER_FILES
        .iter()
        .any(|entry| rel_str == *entry)
}

/// Files with **known pub-data-type violations** that haven't been migrated
/// yet. Empty: the backlog was cleared in PR #118. New entries should not be
/// added here — instead, extract the data types that motivated the entry to
/// a sibling `models.rs`. Kept as a 0-element const so the pattern is
/// available if future migrations need a temporary holding pen.
const PUB_TYPES_CLEANUP_BACKLOG: &[&str] = &[];

/// Migration backlog — kept empty after the one-shot migration that
/// converted every `service.rs` to take `&AuthContext`. **Do not add
/// entries here.** A `service.rs` that takes `&AuthContext` must declare
/// `#[requires]` / `#[requires_any]` / `#[requires_public]`; methods that
/// don't take `&AuthContext` aren't subject to the check by design (token
/// primitives, session lookup, public OAuth start/callback, etc.).
const AUTH_MIGRATION_BACKLOG: &[&str] = &[];

/// Files that currently reference `ObjectId` outside the storage allowlist
/// and have not yet been migrated to `core::public_id::Id<P>`. Each migration
/// commit removes one entry; this list will reach empty when the cutover is
/// done. See issue #156.
///
/// New files inherit enforcement — do not add entries here.
const OBJECT_ID_BACKLOG: &[&str] = &[];

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

// ── AuthContext scope-attribute scanner ──

fn is_auth_migration_backlog(rel_str: &str) -> bool {
    AUTH_MIGRATION_BACKLOG.contains(&rel_str)
}

fn collect_service_rs_relpaths(dir: &std::path::Path, manifest_dir: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_service_rs_relpaths(&path, manifest_dir, out);
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) != Some("service.rs") {
            continue;
        }
        let rel = path.strip_prefix(manifest_dir).unwrap_or(&path);
        out.push(rel.to_string_lossy().replace('\\', "/"));
    }
}

fn scan_service_methods(dir: &std::path::Path, manifest_dir: &str, violations: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            scan_service_methods(&path, manifest_dir, violations);
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) != Some("service.rs") {
            continue;
        }
        let rel = path.strip_prefix(manifest_dir).unwrap_or(&path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if is_auth_migration_backlog(&rel_str) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            if let Some(name) = parse_pub_async_fn_start(lines[i]) {
                let signature = collect_signature(&lines, i);
                if signature_takes_auth_context(&signature)
                    && !has_requires_attribute_above(&lines, i)
                {
                    violations.push(format!(
                        "  {}:{} — pub async fn `{}` takes &AuthContext but lacks #[requires*]",
                        rel_str,
                        i + 1,
                        name,
                    ));
                }
            }
            i += 1;
        }
    }
}

/// Parse `pub async fn <name>(` or `pub(crate) async fn <name>(` and return
/// the name. Returns `None` for any other line.
fn parse_pub_async_fn_start(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }
    let after_pub = if let Some(r) = trimmed.strip_prefix("pub ") {
        r
    } else if let Some(r) = trimmed.strip_prefix("pub(") {
        let close = r.find(')')?;
        r[close + 1..].trim_start()
    } else {
        return None;
    };
    let after_async = after_pub.strip_prefix("async ")?;
    let after_fn = after_async.strip_prefix("fn ")?;
    let name_end = after_fn.find(|c: char| c.is_whitespace() || c == '(')?;
    let name = &after_fn[..name_end];
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Read forward from `start` until the function-signature parens close,
/// joining all lines into one string. Naive paren counting — won't tolerate
/// `(` or `)` inside string literals in signatures (none exist in this
/// codebase; tighten if a real false positive appears).
fn collect_signature(lines: &[&str], start: usize) -> String {
    let mut sig = String::new();
    let mut depth: i32 = 0;
    let mut started = false;
    for line in lines.iter().skip(start) {
        sig.push_str(line);
        sig.push('\n');
        for c in line.chars() {
            if c == '(' {
                depth += 1;
                started = true;
            } else if c == ')' {
                depth -= 1;
            }
        }
        if started && depth == 0 {
            break;
        }
    }
    sig
}

fn signature_takes_auth_context(sig: &str) -> bool {
    // Whitespace-tolerant — accepts `ctx: &AuthContext`, `ctx : &  AuthContext`,
    // `ctx: &mut AuthContext`. Substring check is enough: `AuthContext` is a
    // unique name in the codebase.
    let re = regex::Regex::new(r"\bctx\s*:\s*&\s*(?:mut\s+)?AuthContext\b").unwrap();
    re.is_match(sig)
}

/// Walk backwards from `fn_line` looking for one of the scope attributes.
/// Stops at the first non-blank, non-comment, non-attribute line.
fn has_requires_attribute_above(lines: &[&str], fn_line: usize) -> bool {
    let mut i = fn_line;
    while i > 0 {
        i -= 1;
        let trimmed = lines[i].trim_start();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#[") {
            if rest.starts_with("requires(")
                || rest.starts_with("requires_any(")
                || rest.starts_with("requires_public(")
            {
                return true;
            }
            // Some other attribute (`#[tracing::instrument]`, `#[utoipa::path]`, etc.) — keep scanning.
            continue;
        }
        // Non-attribute statement — we've crossed out of the attribute block.
        break;
    }
    false
}

// ── ObjectId-confinement scanner ──

/// Files allowed to import / reference `ObjectId`. Anything else triggers
/// `object_id_confined_to_storage_layer` (unless on `OBJECT_ID_BACKLOG`).
const OBJECT_ID_ALLOWED_FILES: &[&str] = &[
    "src/app.rs",
    "src/main.rs",
    "src/core/db.rs",
    "src/core/public_id/mod.rs",
    // architecture_tests.rs scans for the word `ObjectId` — it has to mention
    // it (in comments, parser test strings, and the error message).
    "src/architecture_tests.rs",
];

fn is_object_id_allowed(rel_str: &str) -> bool {
    // Allowlisted exact paths.
    if OBJECT_ID_ALLOWED_FILES.contains(&rel_str) {
        return true;
    }
    // Repos and migrations own storage.
    if rel_str.starts_with("src/migrations/") {
        return true;
    }
    if rel_str.ends_with("/repo.rs") {
        return true;
    }
    // Repos with sub-directories like `services/billing/repos/foo.rs` are repo files too.
    if rel_str.contains("/repos/") {
        return true;
    }
    // Sibling test files may reference any type — but ONLY if they sit next
    // to a non-test `.rs` source file in the same directory. Without this
    // gate, anyone could defeat the rule by naming any file `*_tests.rs`.
    // The check covers both:
    //   - `<stem>_tests.rs` next to `<stem>.rs` (e.g. `origin_tests.rs` next to `origin.rs`)
    //   - `<stem>_tests.rs` next to `mod.rs` (sub-module pattern, e.g.
    //     `core/public_id/public_id_tests.rs` next to `core/public_id/mod.rs`)
    if let Some(name) = std::path::Path::new(rel_str)
        .file_name()
        .and_then(|s| s.to_str())
    {
        if name.ends_with("_tests.rs") {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
            let abs = std::path::Path::new(&manifest_dir).join(rel_str);
            if let Some(parent) = abs.parent() {
                // Look for any sibling `.rs` that isn't another test file.
                if let Ok(entries) = std::fs::read_dir(parent) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p == abs {
                            continue;
                        }
                        if p.extension().and_then(|s| s.to_str()) != Some("rs") {
                            continue;
                        }
                        let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        if !n.ends_with("_tests.rs") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn is_object_id_backlog(rel_str: &str) -> bool {
    OBJECT_ID_BACKLOG.contains(&rel_str)
}

/// Walk `dir` recursively. For every `.rs` file not on the allowlist and not
/// on the backlog, record any `ObjectId` (whole-word) reference.
fn scan_for_object_id(dir: &std::path::Path, manifest_dir: &str, violations: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            scan_for_object_id(&path, manifest_dir, violations);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let rel = path.strip_prefix(manifest_dir).unwrap_or(&path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if is_object_id_allowed(&rel_str) || is_object_id_backlog(&rel_str) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (line_num, line) in content.lines().enumerate() {
            // Strip line comments so we don't trip on doc-comments mentioning the type.
            let code = strip_line_comment(line);
            if contains_object_id_word(code) {
                violations.push(format!("  {}:{}", rel_str, line_num + 1));
            }
        }
    }
}

fn file_mentions_object_id(content: &str) -> bool {
    content
        .lines()
        .any(|line| contains_object_id_word(strip_line_comment(line)))
}

/// Whole-word match for `ObjectId`. Rejects substrings (e.g. `MyObjectIdRef`).
fn contains_object_id_word(s: &str) -> bool {
    let needle = "ObjectId";
    let bytes = s.as_bytes();
    let nb = needle.as_bytes();
    let mut i = 0;
    while i + nb.len() <= bytes.len() {
        if &bytes[i..i + nb.len()] == nb {
            let prev_ok =
                i == 0 || !matches!(bytes[i - 1], b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_');
            let next = i + nb.len();
            let next_ok = next == bytes.len()
                || !matches!(bytes[next], b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_');
            if prev_ok && next_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Strip `//`-line comments. Naive: doesn't handle `//` inside string literals.
/// Acceptable for this codebase — no source line has `//` inside a string before
/// a meaningful `ObjectId` reference.
fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
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

    use super::{contains_object_id_word, is_object_id_allowed, strip_line_comment};

    #[test]
    fn word_match_accepts_standalone() {
        assert!(contains_object_id_word("    pub id: ObjectId,"));
        assert!(contains_object_id_word("fn foo(id: ObjectId) {"));
        assert!(contains_object_id_word(
            "pub tenant_id: mongodb::bson::oid::ObjectId,"
        ));
        assert!(contains_object_id_word("Vec<ObjectId>"));
        assert!(contains_object_id_word("Option<ObjectId>"));
        assert!(contains_object_id_word("ObjectId::parse_str(s)"));
    }

    #[test]
    fn word_match_rejects_substring() {
        assert!(!contains_object_id_word("MyObjectIdRef"));
        assert!(!contains_object_id_word("ObjectIds"));
        assert!(!contains_object_id_word("_ObjectId"));
        assert!(!contains_object_id_word("foo123ObjectId"));
    }

    #[test]
    fn strip_line_comment_works() {
        assert_eq!(
            strip_line_comment("pub x: i32, // ObjectId here"),
            "pub x: i32, "
        );
        assert_eq!(strip_line_comment("// just a doc ObjectId"), "");
        assert_eq!(strip_line_comment("pub id: ObjectId,"), "pub id: ObjectId,");
    }

    #[test]
    fn allowlist_includes_repos() {
        assert!(is_object_id_allowed("src/services/affiliates/repo.rs"));
        assert!(is_object_id_allowed("src/services/auth/users/repo.rs"));
        assert!(is_object_id_allowed(
            "src/services/billing/repos/event_counters.rs"
        ));
    }

    #[test]
    fn allowlist_includes_migrations_and_bootstrap() {
        assert!(is_object_id_allowed("src/migrations/m001_auth_split.rs"));
        assert!(is_object_id_allowed("src/core/db.rs"));
        assert!(is_object_id_allowed("src/core/public_id/mod.rs"));
        assert!(is_object_id_allowed("src/app.rs"));
        assert!(is_object_id_allowed("src/main.rs"));
    }

    #[test]
    fn allowlist_includes_sibling_tests() {
        assert!(is_object_id_allowed("src/services/billing/quota_tests.rs"));
        assert!(is_object_id_allowed("src/services/links/service_tests.rs"));
    }

    #[test]
    fn allowlist_excludes_transports_and_services() {
        assert!(!is_object_id_allowed("src/api/affiliates/routes.rs"));
        assert!(!is_object_id_allowed("src/services/affiliates/models.rs"));
        assert!(!is_object_id_allowed("src/services/affiliates/service.rs"));
        assert!(!is_object_id_allowed("src/mcp/models.rs"));
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
