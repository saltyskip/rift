use assert_cmd::Command;
use predicates::prelude::*;

fn rift() -> Command {
    Command::cargo_bin("rift").unwrap()
}

// ── Version ──

#[test]
fn version_flag() {
    rift()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rift 0.1.0"));
}

// ── Help ──

#[test]
fn help_shows_all_top_level_commands() {
    rift()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("logout"))
        .stdout(predicate::str::contains("whoami"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("links"))
        .stdout(predicate::str::contains("apps"))
        .stdout(predicate::str::contains("domains"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn links_help_shows_subcommands() {
    rift()
        .args(["links", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("test"));
}

#[test]
fn apps_help_shows_subcommands() {
    rift()
        .args(["apps", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"));
}

#[test]
fn domains_help_shows_subcommands() {
    rift()
        .args(["domains", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("setup"));
}

// ── Unknown command ──

#[test]
fn unknown_command_fails() {
    rift()
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

// ── Completions ──

#[test]
fn completions_zsh() {
    rift()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef rift"));
}

#[test]
fn completions_bash() {
    rift()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_rift"));
}

#[test]
fn completions_fish() {
    rift()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c rift"));
}

// ── Logout on fresh machine (no config) ──

#[test]
fn logout_json_when_not_logged_in() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = rift();
    cmd.args(["logout", "--json"]).env("HOME", dir.path());
    if cfg!(not(target_os = "macos")) {
        cmd.env("XDG_CONFIG_HOME", dir.path().join(".config"));
    }
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("not_logged_in"));
}
