use std::collections::hash_map::DefaultHasher;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const SMALL_OUTPUT_THRESHOLD: usize = 80;
const MAX_OUTPUT: usize = 16 * 1024 * 1024;
const SESSION_TTL_SECS: u64 = 30;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SessionEntry {
    cmd_hash: u64,
    out_hash: u64,
    timestamp: u64,
    output: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct SessionState {
    entries: Vec<SessionEntry>,
}

pub fn run_proxy(cmd: &[String]) -> ExitCode {
    if cmd.is_empty() {
        eprintln!("usage: ztk run <cmd> [args...]");
        return ExitCode::from(1);
    }

    let rendered = cmd.join(" ");
    let result = match exec_command(cmd) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("ztk: error: {err}");
            return ExitCode::from(1);
        }
    };

    let filtered = apply_filters(&rendered, &result.stdout);
    let final_output = maybe_apply_session(&rendered, &filtered);
    print_output(&final_output);
    let _ = log_savings(&rendered, result.stdout.len(), final_output.len(), result.exit_code);

    ExitCode::from(result.exit_code)
}

struct ExecResult {
    stdout: String,
    exit_code: u8,
}

fn exec_command(cmd: &[String]) -> anyhow::Result<ExecResult> {
    let output = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;

    let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if stdout.len() > MAX_OUTPUT {
        stdout = "[ztk: command output exceeded 16MB cap, raw output suppressed]\n".to_string();
    }

    let exit_code = output.status.code().unwrap_or(1).clamp(0, 255) as u8;
    Ok(ExecResult { stdout, exit_code })
}

fn apply_filters(cmd: &str, stdout: &str) -> String {
    if stdout.len() < SMALL_OUTPUT_THRESHOLD {
        return stdout.to_string();
    }

    if cmd.starts_with("ls") {
        return summarize_lines(stdout, 20, "directory entries");
    }
    if cmd.starts_with("find") || cmd.starts_with("grep") {
        return summarize_lines(stdout, 40, "matches");
    }
    if cmd.starts_with("cargo test") || cmd.starts_with("pytest") || cmd.starts_with("go test") {
        return summarize_test_output(stdout);
    }
    if cmd.starts_with("git diff") {
        return summarize_diff(stdout);
    }
    if cmd.starts_with("cat") {
        return summarize_code(stdout);
    }

    dedupe_repeated_lines(stdout)
}

fn summarize_lines(stdout: &str, keep: usize, label: &str) -> String {
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() <= keep {
        return stdout.to_string();
    }
    let mut out = format!("[{label}: {} total, showing first {keep}]\n", lines.len());
    for line in lines.iter().take(keep) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn summarize_test_output(stdout: &str) -> String {
    let mut failures = Vec::new();
    let mut summary = None;
    for line in stdout.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("fail") || lower.contains("error") || lower.contains("panic") {
            failures.push(line);
        }
        if lower.contains("test result") || lower.contains("passed") || lower.contains("failed") {
            summary = Some(line);
        }
    }
    if failures.is_empty() {
        return summary
            .map(|s| format!("{}\n", s.trim()))
            .unwrap_or_else(|| "[tests passed]\n".to_string());
    }
    let mut out = String::from("[test failures]\n");
    for line in failures.into_iter().take(20) {
        out.push_str(line);
        out.push('\n');
    }
    if let Some(summary) = summary {
        out.push_str(summary);
        out.push('\n');
    }
    out
}

fn summarize_diff(stdout: &str) -> String {
    let mut out = String::new();
    for line in stdout.lines() {
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") || line.starts_with('+') || line.starts_with('-') {
            out.push_str(line);
            out.push('\n');
        }
    }
    if out.is_empty() {
        stdout.to_string()
    } else {
        out
    }
}

fn summarize_code(stdout: &str) -> String {
    let mut out = String::new();
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("mod ")
            || trimmed.starts_with("use ")
        {
            out.push_str(line);
            out.push('\n');
        }
    }
    if out.is_empty() {
        summarize_lines(stdout, 80, "file preview")
    } else {
        out
    }
}

fn dedupe_repeated_lines(stdout: &str) -> String {
    let mut out = String::new();
    let mut prev = "";
    let mut count = 0usize;
    for line in stdout.lines() {
        if line == prev {
            count += 1;
            continue;
        }
        if count > 1 {
            out.push_str(&format!("[previous line repeated {}x]\n", count));
        }
        if !prev.is_empty() {
            out.push_str(prev);
            out.push('\n');
        }
        prev = line;
        count = 1;
    }
    if count > 1 {
        out.push_str(&format!("[previous line repeated {}x]\n", count));
    }
    if !prev.is_empty() {
        out.push_str(prev);
        out.push('\n');
    }
    if out.is_empty() { stdout.to_string() } else { out }
}

fn maybe_apply_session(cmd: &str, filtered: &str) -> String {
    let Some(path) = session_path() else {
        return filtered.to_string();
    };

    let cmd_hash = stable_hash(cmd);
    let out_hash = stable_hash(filtered);
    let now = now_secs();

    let mut state = load_session(&path).unwrap_or_default();
    if let Some(entry) = state.entries.iter_mut().find(|e| e.cmd_hash == cmd_hash) {
        if entry.out_hash == out_hash && now.saturating_sub(entry.timestamp) <= SESSION_TTL_SECS {
            return format!("[unchanged output omitted; last shown {}s ago]\n", now.saturating_sub(entry.timestamp));
        }
        entry.out_hash = out_hash;
        entry.timestamp = now;
        entry.output = filtered.to_string();
    } else {
        state.entries.push(SessionEntry {
            cmd_hash,
            out_hash,
            timestamp: now,
            output: filtered.to_string(),
        });
    }
    let _ = save_session(&path, &state);
    filtered.to_string()
}

fn print_output(output: &str) {
    print!("{output}");
    if !output.ends_with('\n') {
        println!();
    }
}

fn log_savings(command: &str, original: usize, filtered: usize, exit_code: u8) -> anyhow::Result<()> {
    let Some(mut path) = data_dir() else {
        return Ok(());
    };
    fs::create_dir_all(&path)?;
    path.push("savings.log");

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let savings = if original > filtered { ((original - filtered) * 100) / original } else { 0 };
    writeln!(file, "{}\t{}\t{}\t{}\t{}%\texit={}", now_secs(), command, original, filtered, savings, exit_code)?;
    Ok(())
}

fn load_session(path: &PathBuf) -> anyhow::Result<SessionState> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn save_session(path: &PathBuf, state: &SessionState) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn session_path() -> Option<PathBuf> {
    let mut path = std::env::temp_dir();
    path.push("ztk-session.json");
    Some(path)
}

fn data_dir() -> Option<PathBuf> {
    let mut dir = dirs::home_dir()?;
    dir.push(".local/share/ztk");
    Some(dir)
}

fn stable_hash(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_summary_keeps_hunks() {
        let input = "diff --git a b\n--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n";
        let out = summarize_diff(input);
        assert!(out.contains("+new"));
        assert!(!out.contains("diff --git"));
    }
}