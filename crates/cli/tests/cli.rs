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

/// Same as `isolated_cmd`, but also returns a temp directory the test can
/// write real media files into and register as a library root — separate
/// from the CLI's own data dir.
fn isolated_cmd_with_media_dir() -> (Command, tempfile::TempDir, tempfile::TempDir) {
    let (cmd, home_dir) = isolated_cmd();
    let media_dir = tempfile::tempdir().unwrap();
    (cmd, home_dir, media_dir)
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

/// `docs/28-release-checklist.md`'s "plain output works without
/// colour" accessibility gate — no color crate is a dependency
/// anywhere in this workspace, so this is already structurally true;
/// this locks it in as a real regression guard rather than leaving it
/// untested.
#[test]
fn cli_text_output_never_contains_ansi_escape_codes() {
    let (mut cmd, _dir) = isolated_cmd();
    let output = cmd.arg("doctor").output().unwrap();
    assert!(output.status.success());
    assert!(
        !output.stdout.contains(&0x1b),
        "CLI text output must never contain ANSI escape sequences"
    );
}

#[test]
fn doctor_json_output_includes_schema_version() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.args(["--output", "json", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\":1"))
        .stdout(predicate::str::contains(format!(
            "\"applied_migrations\":{}",
            database::migrations::MIGRATIONS.len()
        )));
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
        .stdout(predicate::str::contains(format!(
            "\"applied_migrations\":{}",
            database::migrations::MIGRATIONS.len()
        )));
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

#[test]
fn library_add_list_and_status() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("added library root"));

    let (mut cmd, _home2) = isolated_cmd();
    cmd.env("HOME", home.path());
    cmd.env("XDG_DATA_HOME", home.path().join("data"));
    cmd.arg("library")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains(media.path().to_str().unwrap()));
}

#[test]
fn library_scan_requires_a_registered_root() {
    let (mut cmd, _home, media) = isolated_cmd_with_media_dir();
    cmd.arg("library")
        .arg("scan")
        .arg("--path")
        .arg(media.path())
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("library add"));
}

#[test]
fn library_remove_refuses_without_yes() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("library")
        .arg("remove")
        .arg("00000000-0000-0000-0000-000000000000")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn scan_search_favorite_and_item_show_end_to_end() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("clip.mp4"), b"fake video bytes").unwrap();

    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd
        .arg("library")
        .arg("scan")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 added"));

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:video"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    assert_eq!(json["total"], 1);
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let mut fav_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut fav_cmd);
    fav_cmd
        .arg("favorite")
        .arg("add")
        .arg(&item_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("favorited"));

    let mut show_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut show_cmd);
    show_cmd
        .arg("item")
        .arg("show")
        .arg(&item_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("favorite: true"));
}

#[test]
fn item_open_no_launch_resolves_a_video_without_launching_a_player() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("clip.mp4"), b"fake video bytes").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:video"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let mut open_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut open_cmd);
    open_cmd
        .args(["--output", "json", "item", "open", &item_id, "--no-launch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\":\"external_player\""));
}

#[test]
fn item_progress_records_completion_past_the_threshold() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("clip.mp4"), b"fake video bytes").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:video"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let mut progress_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut progress_cmd);
    progress_cmd
        .args([
            "--output",
            "json",
            "item",
            "progress",
            &item_id,
            "--json",
            r#"{"progress_type":"time_based","position_ms":9500,"duration_ms":10000}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"completed\":true"));
}

#[test]
fn item_pages_lists_a_cbz_archive() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    let cbz_path = media.path().join("book.cbz");
    let file = std::fs::File::create(&cbz_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("000.jpg", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"page bytes").unwrap();
    writer.finish().unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:comic"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let mut pages_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut pages_cmd);
    pages_cmd
        .arg("item")
        .arg("pages")
        .arg(&item_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("1 page(s)"));
}

#[test]
fn item_read_prints_sanitized_story_content() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("tale.md"), "# Once\nUpon a time.\n").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:story"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let mut read_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut read_cmd);
    read_cmd
        .arg("item")
        .arg("read")
        .arg(&item_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("Upon a time."));
}

#[test]
fn collection_create_list_and_item_membership() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("pic.png"), b"fake png bytes").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:image"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let mut create_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut create_cmd);
    let output = create_cmd
        .args(["--output", "json", "collection", "create", "Later"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let collection_id = json["id"].as_str().unwrap().to_string();

    let mut add_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut add_cmd);
    add_cmd
        .arg("collection")
        .arg("add")
        .arg(&item_id)
        .arg("--to")
        .arg(&collection_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("added to"));

    let mut list_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut list_cmd);
    list_cmd
        .arg("collection")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Later"));
}

#[test]
fn diagnostics_bundle_omits_titles_and_prints_to_stdout() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("clip.mp4"), b"fake video bytes").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut bundle_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut bundle_cmd);
    bundle_cmd
        .args(["diagnostics", "bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"item_counts_by_media_type\""))
        .stdout(predicate::str::contains("clip").not())
        .stdout(predicate::str::contains(media.path().to_str().unwrap()).not());
}

#[test]
fn diagnostics_bundle_writes_to_a_file_when_requested() {
    let (mut cmd, _dir) = isolated_cmd();
    let bundle_path = _dir.path().join("bundle.json");
    cmd.args(["diagnostics", "bundle", "--file"])
        .arg(&bundle_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("support bundle written to"));
    assert!(bundle_path.exists());
}

#[test]
fn db_backup_and_restore_round_trip() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("clip.mp4"), b"fake video bytes").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:video"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let backup_path = home.path().join("backup.db");
    let mut backup_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut backup_cmd);
    backup_cmd
        .arg("db")
        .arg("backup")
        .arg(&backup_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("backup written to"));
    assert!(backup_path.exists());

    // Change state after the backup...
    let mut fav_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut fav_cmd);
    fav_cmd
        .arg("favorite")
        .arg("add")
        .arg(&item_id)
        .assert()
        .success();

    // ...then restore, which must revert to the pre-backup state.
    let mut restore_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut restore_cmd);
    restore_cmd
        .arg("db")
        .arg("restore")
        .arg(&backup_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("restart"));

    let mut show_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut show_cmd);
    show_cmd
        .arg("item")
        .arg("show")
        .arg(&item_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("favorite: false"));
}

#[test]
fn item_pin_toggle_round_trips() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("pic.png"), b"fake png bytes").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    let output = search_cmd
        .args(["--output", "json", "search", "type:image"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let item_id = json["items"][0]["item_id"].as_str().unwrap().to_string();

    let mut pin_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut pin_cmd);
    pin_cmd
        .arg("item")
        .arg("pin")
        .arg(&item_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("pinned"));

    let mut show_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut show_cmd);
    show_cmd
        .args(["--output", "json", "item", "show", &item_id])
        .assert()
        .success();

    let mut unpin_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut unpin_cmd);
    unpin_cmd
        .arg("item")
        .arg("pin")
        .arg(&item_id)
        .arg("--unpin")
        .assert()
        .success()
        .stdout(predicate::str::contains("unpinned"));
}

#[test]
fn db_cache_status_quota_and_enforce_quota_round_trip() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.args(["--output", "json", "db", "cache-status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_bytes\":0"))
        .stdout(predicate::str::contains("\"quota_bytes\":null"));

    let (mut cmd2, _dir2) = isolated_cmd();
    cmd2.env("HOME", _dir.path());
    cmd2.env("XDG_DATA_HOME", _dir.path().join("data"));
    cmd2.args(["db", "cache-quota", "1024"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache quota set to 1024 bytes"));

    let mut status_cmd = Command::cargo_bin("veloura").unwrap();
    status_cmd.env("HOME", _dir.path());
    status_cmd.env("XDG_DATA_HOME", _dir.path().join("data"));
    status_cmd
        .args(["--output", "json", "db", "cache-status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"quota_bytes\":1024"));

    let mut enforce_cmd = Command::cargo_bin("veloura").unwrap();
    enforce_cmd.env("HOME", _dir.path());
    enforce_cmd.env("XDG_DATA_HOME", _dir.path().join("data"));
    enforce_cmd
        .arg("db")
        .arg("cache-enforce-quota")
        .assert()
        .success()
        .stdout(predicate::str::contains("evicted 0 file(s)"));

    let mut clear_cmd = Command::cargo_bin("veloura").unwrap();
    clear_cmd.env("HOME", _dir.path());
    clear_cmd.env("XDG_DATA_HOME", _dir.path().join("data"));
    clear_cmd
        .args(["db", "cache-quota", "--clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache quota cleared"));
}

#[test]
fn db_cache_quota_requires_either_a_value_or_clear() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.args(["db", "cache-quota"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("pass a byte value"));
}

#[test]
fn db_restore_rejects_an_invalid_backup_file() {
    let (mut cmd, _dir) = isolated_cmd();
    let bogus_path = _dir.path().join("bogus.db");
    std::fs::write(&bogus_path, b"not a database").unwrap();
    cmd.arg("db")
        .arg("restore")
        .arg(&bogus_path)
        .assert()
        .failure();
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn plugin_validate_reports_a_valid_manifest() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("plugin")
        .arg("validate")
        .arg(fixture_path("valid_plugin.yaml"))
        .assert()
        .success()
        .stdout(predicate::str::contains("org.example.connector"))
        .stdout(predicate::str::contains("valid: yes"));
}

#[test]
fn plugin_validate_reports_issues_for_an_invalid_manifest() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("plugin")
        .arg("validate")
        .arg(fixture_path("invalid_plugin.yaml"))
        .assert()
        .failure()
        .code(2)
        .stdout(predicate::str::contains("valid: no"))
        .stdout(predicate::str::contains("api_version"));
}

#[test]
fn plugin_registry_add_list_and_set_status_round_trip() {
    let (mut cmd, home) = isolated_cmd();
    cmd.arg("plugin")
        .arg("registry-add")
        .arg(fixture_path("valid_plugin.yaml"))
        .arg("--status")
        .arg("beta")
        .assert()
        .success()
        .stdout(predicate::str::contains("added org.example.connector"));

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut list_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut list_cmd);
    list_cmd
        .arg("plugin")
        .arg("registry-list")
        .assert()
        .success()
        .stdout(predicate::str::contains("org.example.connector"))
        .stdout(predicate::str::contains("Beta"));

    let mut status_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut status_cmd);
    status_cmd
        .arg("plugin")
        .arg("registry-set-status")
        .arg("org.example.connector")
        .arg("--status")
        .arg("disabled")
        .assert()
        .success();

    let mut list_cmd2 = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut list_cmd2);
    list_cmd2
        .arg("plugin")
        .arg("registry-list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Disabled"));
}

#[test]
fn plugin_registry_list_on_a_fresh_data_dir_is_empty() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("plugin")
        .arg("registry-list")
        .assert()
        .success()
        .stdout(predicate::str::contains("no plugins registered"));
}

#[test]
fn plugin_registry_set_status_on_an_unknown_id_is_not_found() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("plugin")
        .arg("registry-set-status")
        .arg("does.not.exist")
        .arg("--status")
        .arg("disabled")
        .assert()
        .failure()
        .code(3);
}

const FEED_CONNECTOR_ID: &str = "00000000-0000-0000-0000-000000000001";
const LOCAL_FILESYSTEM_CONNECTOR_ID: &str = "00000000-0000-0000-0000-000000000000";

#[test]
fn source_list_on_a_fresh_data_dir_is_empty() {
    let (mut cmd, _dir) = isolated_cmd();
    cmd.arg("source")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("no sources configured"));
}

#[test]
fn source_add_list_enable_disable_and_remove_round_trip() {
    let (mut cmd, home) = isolated_cmd();
    let output = cmd
        .args([
            "--output",
            "json",
            "source",
            "add",
            FEED_CONNECTOR_ID,
            "My Feed",
        ])
        .args([
            "--config-json",
            r#"{"url":"https://example.test/feed.xml"}"#,
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let source_id = json["id"].as_str().unwrap().to_string();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut list_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut list_cmd);
    list_cmd
        .arg("source")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("My Feed"));

    let mut disable_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut disable_cmd);
    disable_cmd
        .arg("source")
        .arg("disable")
        .arg(&source_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("disabled"));

    let mut enable_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut enable_cmd);
    enable_cmd
        .arg("source")
        .arg("enable")
        .arg(&source_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("enabled"));

    let mut remove_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut remove_cmd);
    remove_cmd
        .arg("source")
        .arg("remove")
        .arg(&source_id)
        .assert()
        .success();

    let mut list_cmd2 = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut list_cmd2);
    list_cmd2
        .arg("source")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("no sources configured"));
}

#[test]
fn source_health_check_and_browse_for_the_local_connector() {
    let (mut cmd, home, media) = isolated_cmd_with_media_dir();
    std::fs::write(media.path().join("clip.mp4"), b"fake video bytes").unwrap();
    cmd.arg("library")
        .arg("add")
        .arg(media.path())
        .assert()
        .success();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let mut scan_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut scan_cmd);
    scan_cmd.arg("library").arg("scan").assert().success();

    let mut add_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut add_cmd);
    let output = add_cmd
        .args([
            "--output",
            "json",
            "source",
            "add",
            LOCAL_FILESYSTEM_CONNECTOR_ID,
            "Local Library",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let source_id = json["id"].as_str().unwrap().to_string();

    let mut health_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut health_cmd);
    health_cmd
        .arg("source")
        .arg("health-check")
        .arg(&source_id)
        .assert()
        .success()
        .stdout(predicate::str::contains("Healthy"));

    let mut browse_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut browse_cmd);
    browse_cmd
        .arg("source")
        .arg("browse")
        .arg(&source_id)
        .args(["--query", "type:video"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Success"));
}

#[test]
fn source_import_creates_a_searchable_item() {
    let (mut cmd, home) = isolated_cmd();
    let output = cmd
        .args([
            "--output",
            "json",
            "source",
            "add",
            FEED_CONNECTOR_ID,
            "My Feed",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let source_id = json["id"].as_str().unwrap().to_string();

    let env_pair = |c: &mut Command| {
        c.env("HOME", home.path());
        c.env("XDG_DATA_HOME", home.path().join("data"));
    };

    let remote_item_json = serde_json::json!({
        "source_item_id": "guid-1",
        "title": "Imported Story",
        "description": null,
        "canonical_url": "https://example.test/story",
        "tags": ["fiction"],
        "media_type": "story",
        "thumbnail_url": null,
    })
    .to_string();

    let mut import_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut import_cmd);
    import_cmd
        .arg("source")
        .arg("import")
        .arg(&source_id)
        .args(["--json", &remote_item_json])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported as item"));

    let mut search_cmd = Command::cargo_bin("veloura").unwrap();
    env_pair(&mut search_cmd);
    search_cmd
        .args(["--output", "json", "search", "Imported"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total\":1"));
}
