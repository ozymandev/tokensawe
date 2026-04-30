use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const BEGIN_MARKER: &str = "# ztk managed start";
const END_MARKER: &str = "# ztk managed end";
const MANAGED_BLOCK: &str = "# ztk managed start\n[ztk]\nenabled = true\nsession_ttl_secs = 30\n# ztk managed end\n";

pub fn run_init(global: bool) -> ExitCode {
    match install_hook(global) {
        Ok(path) => {
            println!("ztk: Claude hook configuration written to {}", path.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ztk: init failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub fn run_add(file: &str) -> ExitCode {
    match add_managed_block(Path::new(file)) {
        Ok(ActionResult::Updated(path)) => {
            println!("ztk: added managed settings to {}", path.display());
            ExitCode::SUCCESS
        }
        Ok(ActionResult::Unchanged(path)) => {
            println!("ztk: managed settings already present in {}", path.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ztk: add failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub fn run_rm(file: &str) -> ExitCode {
    match remove_managed_block(Path::new(file)) {
        Ok(ActionResult::Updated(path)) => {
            println!("ztk: removed managed settings from {}", path.display());
            ExitCode::SUCCESS
        }
        Ok(ActionResult::Unchanged(path)) => {
            println!("ztk: managed settings not present in {}", path.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ztk: rm failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub fn run_rewrite() -> ExitCode {
    match std::io::read_to_string(std::io::stdin()) {
        Ok(input) => {
            print!("{input}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ztk: rewrite failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn install_hook(global: bool) -> anyhow::Result<PathBuf> {
    let mut path = if global {
        PathBuf::from("/etc/claude-code")
    } else {
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("home directory not found"))?
    };

    if !global {
        path.push(".config/claude-code");
    }
    fs::create_dir_all(&path)?;
    path.push("ztk-hook.json");

    let payload = serde_json::json!({
        "pre_tool_use": {
            "command": "ztk rewrite"
        }
    });
    fs::write(&path, serde_json::to_vec_pretty(&payload)?)?;
    Ok(path)
}

#[derive(Debug, PartialEq, Eq)]
enum ActionResult {
    Updated(PathBuf),
    Unchanged(PathBuf),
}

fn add_managed_block(path: &Path) -> anyhow::Result<ActionResult> {
    let mut content = read_or_empty(path)?;
    if has_managed_block(&content) {
        return Ok(ActionResult::Unchanged(path.to_path_buf()));
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(MANAGED_BLOCK);
    fs::write(path, content)?;
    Ok(ActionResult::Updated(path.to_path_buf()))
}

fn remove_managed_block(path: &Path) -> anyhow::Result<ActionResult> {
    let content = read_or_empty(path)?;
    let Some(start) = content.find(BEGIN_MARKER) else {
        return Ok(ActionResult::Unchanged(path.to_path_buf()));
    };
    let Some(end_marker_pos) = content[start..].find(END_MARKER) else {
        return Ok(ActionResult::Unchanged(path.to_path_buf()));
    };
    let end = start + end_marker_pos + END_MARKER.len();
    let mut new_content = String::with_capacity(content.len());
    new_content.push_str(&content[..start]);

    let remainder = content[end..].strip_prefix('\n').unwrap_or(&content[end..]);
    new_content.push_str(remainder);

    while new_content.contains("\n\n\n") {
        new_content = new_content.replace("\n\n\n", "\n\n");
    }

    fs::write(path, new_content)?;
    Ok(ActionResult::Updated(path.to_path_buf()))
}

fn has_managed_block(content: &str) -> bool {
    content.contains(BEGIN_MARKER) && content.contains(END_MARKER)
}

fn read_or_empty(path: &Path) -> anyhow::Result<String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn add_managed_block_is_idempotent() {
        let path = temp_file("add-idempotent");

        let first = add_managed_block(&path).unwrap();
        let second = add_managed_block(&path).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert_eq!(first, ActionResult::Updated(path.clone()));
        assert_eq!(second, ActionResult::Unchanged(path.clone()));
        assert_eq!(content.matches(BEGIN_MARKER).count(), 1);
    }

    #[test]
    fn remove_managed_block_preserves_other_content() {
        let path = temp_file("rm-managed");
        fs::write(&path, format!("before\n{}after\n", MANAGED_BLOCK)).unwrap();

        let result = remove_managed_block(&path).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert_eq!(result, ActionResult::Updated(path.clone()));
        assert_eq!(content, "before\nafter\n");
    }

    fn temp_file(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        path.push(format!("tokensawe-{label}-{nanos}.toml"));
        path
    }
}
