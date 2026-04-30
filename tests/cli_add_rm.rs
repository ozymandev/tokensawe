use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_tokensawe")
}

fn temp_path(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("tokensawe-{label}-{nanos}.txt"));
    path
}

#[test]
fn add_command_updates_file_and_is_idempotent() {
    let path = temp_path("add-cli");
    fs::write(&path, "name = \"demo\"\n").unwrap();

    let first = Command::new(bin()).arg("add").arg(&path).output().unwrap();
    assert!(first.status.success());
    let stdout = String::from_utf8_lossy(&first.stdout);
    assert!(stdout.contains("added managed settings"));

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("# ztk managed start"));
    assert!(content.contains("[ztk]"));

    let second = Command::new(bin()).arg("add").arg(&path).output().unwrap();
    assert!(second.status.success());
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("already present"));

    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content.matches("# ztk managed start").count(), 1);
}

#[test]
fn rm_command_removes_only_managed_block() {
    let path = temp_path("rm-cli");
    fs::write(
        &path,
        "before = true\n# ztk managed start\n[ztk]\nenabled = true\nsession_ttl_secs = 30\n# ztk managed end\nafter = true\n",
    )
    .unwrap();

    let output = Command::new(bin()).arg("rm").arg(&path).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("removed managed settings"));

    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "before = true\nafter = true\n");
}

#[test]
fn usage_command_prints_add_and_rm_help() {
    let output = Command::new(bin()).arg("usage").output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Add filter/session settings to a config file"));
    assert!(stdout.contains("Remove filter/session settings from a config file"));
    assert!(stdout.contains("Usage:"));
}
