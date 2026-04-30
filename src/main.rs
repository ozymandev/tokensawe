use std::process::ExitCode;
use clap::{CommandFactory, Parser, Subcommand};

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
        #[arg(long, short = 'g')]
        global: bool,
    },
    /// Add filter/session settings to a config file
    Add {
        /// Config file to update
        #[arg(value_name = "FILE")]
        file: String,
    },
    /// Remove filter/session settings from a config file
    Rm {
        /// Config file to update
        #[arg(value_name = "FILE")]
        file: String,
    },
    /// Print usage/help text
    Usage,
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
        Commands::Add { file } => hooks::run_add(&file),
        Commands::Rm { file } => hooks::run_rm(&file),
        Commands::Usage => {
            Cli::command().print_help().ok();
            println!();
            ExitCode::SUCCESS
        }
        Commands::Rewrite => hooks::run_rewrite(),
        Commands::Stats => stats::run_stats(),
        Commands::Version => {
            println!("tokensawe {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
    }
}
