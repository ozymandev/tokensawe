# tokensawe

Token compression proxy for AI coding assistants.

Same behavior as the original Zig implementation, rewritten in Rust for:
- smaller binary (~200 KB Release build)
- faster startup
- same algorithms & cache logic
- drop-in compatibility

## Installation
```bash
# Via cargo (once published)
cargo install ztk

# Manual
curl -L https://github.com/ozymandev/tokensawe/releases/latest/download/ztk-x86_64-unknown-linux-gnu -o /usr/local/bin/ztk
chmod +x /usr/local/bin/ztk
```

## Quick start
```bash
# Initialize global Claude Code hook
ztk init -g

# Run commands through ztk
ztk run git diff
ztk run ls -la src/
ztk run cargo test

# Review your savings
ztk stats
```

## Commands
- `ztk run <cmd> [args...]` – Execute and filter output
- `ztk init [-g]` – Install Claude Code PreToolUse hook
- `ztk rewrite` – Hook entry point (reads stdin)
- `ztk stats` – Show savings TUI
- `ztk version` – Print version

See original project docs for details on filters, session caching, and internals.

## License
MIT