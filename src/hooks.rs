use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

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
