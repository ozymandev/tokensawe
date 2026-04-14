use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

pub fn run_stats() -> ExitCode {
    let Some(path) = log_path() else {
        eprintln!("ztk: no savings log yet. Run some commands with `ztk run ...` first.");
        return ExitCode::SUCCESS;
    };

    let Ok(contents) = fs::read_to_string(&path) else {
        eprintln!("ztk: no savings log yet. Run some commands with `ztk run ...` first.");
        return ExitCode::SUCCESS;
    };

    let mut total_original = 0usize;
    let mut total_filtered = 0usize;
    let mut by_command: HashMap<String, (usize, usize, usize)> = HashMap::new();

    for line in contents.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 6 {
            continue;
        }
        let command = parts[1].to_string();
        let original = parts[2].parse::<usize>().unwrap_or(0);
        let filtered = parts[3].parse::<usize>().unwrap_or(0);

        total_original += original;
        total_filtered += filtered;

        let key = command.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
        let entry = by_command.entry(key).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += original;
        entry.2 += filtered;
    }

    let total_saved = total_original.saturating_sub(total_filtered);
    let total_pct = if total_original == 0 { 0 } else { total_saved * 100 / total_original };

    println!("ztk savings");
    println!("===========");
    println!("Commands run : {}", by_command.values().map(|v| v.0).sum::<usize>());
    println!("Original     : {} bytes", total_original);
    println!("Filtered     : {} bytes", total_filtered);
    println!("Saved        : {} bytes ({}%)", total_saved, total_pct);
    println!();
    println!("Top commands:");

    let mut rows: Vec<_> = by_command.into_iter().collect();
    rows.sort_by_key(|(_, (_, original, filtered))| std::cmp::Reverse(original.saturating_sub(*filtered)));

    for (command, (count, original, filtered)) in rows.into_iter().take(10) {
        let saved = original.saturating_sub(filtered);
        let pct = if original == 0 { 0 } else { saved * 100 / original };
        println!("- {command}: {count} runs, saved {saved} bytes ({pct}%)");
    }

    ExitCode::SUCCESS
}

fn log_path() -> Option<PathBuf> {
    let mut dir = dirs::home_dir()?;
    dir.push(".local/share/ztk/savings.log");
    Some(dir)
}
