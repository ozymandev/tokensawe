use std::process::ExitCode;
use clap::{Parser, Subcommand};

mod proxy;
mod stats;
mod hooks;

#[derive(Parser)]
#[command(name = "tokensawe", version, about = "Token compression proxy for AI coding assistants")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a command and emit compact output
    Run {
        /// Command to execute
        #[arg(last = true)]
        cmd: Vec<String>,
    },
    /// Install Claude Code PreToolUse hook
    Init {
        /// Install globally (affects all users)
        #[arg(long)]
        global: bool,
    },
    /// PreToolUse hook handler (reads stdin)
    Rewrite,
    /// Print savings stats
    Stats,
    /// Print version
    Version,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { cmd } => proxy::run_proxy(&cmd),
        Commands::Init { global } => hooks::run_init(global),
        Commands::Rewrite => hooks::run_rewrite(),
        Commands::Stats => stats::run_stats(),
        Commands::Version => {
            println!("tokensawe {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
    }
}