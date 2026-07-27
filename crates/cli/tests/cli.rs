use assert_cmd::Command;
use predicates::prelude::*;

/// Points the CLI's OS-data-dir resolution at a fresh temp directory so
/// tests never touch (or collide with) a real user's Veloura data, or
/// each other when run in parallel.
fn isolated_cmd() -> (Command, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("veloura").unwrap();
    cmd.env("HOME", dir.path());
    cmd.env("XDG_DATA_HOME", dir.path().join("data"));
    (cmd, dir)
}

#[test]
fn doctor_reports_ok_in_text_mode() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("veloura doctor"))
        .stdout(predicate::str::contains("status:"));
}

#[test]
fn doctor_json_output_includes_schema_version() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.args(["--output", "json", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\":1"))
        .stdout(predicate::str::contains("\"applied_migrations\":1"));
}

#[test]
fn db_check_reports_on_a_fresh_database() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.args(["db", "check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("database ok"));
}

#[test]
fn db_check_json_output_is_well_formed() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.args(["--output", "json", "db", "check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"applied_migrations\":1"));
}

#[test]
fn version_flag_exits_zero() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("--version").assert().success();
}

#[test]
fn quiet_suppresses_text_output_but_still_succeeds() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.args(["--quiet", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}
