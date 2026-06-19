# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Common Commands

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Run the bot in the foreground (requires .env)
cargo run -p rustvoice -- run

# Run a single test in a specific crate
cargo test -p bot test_name

# SeaORM: generate a new migration
sea-orm-cli migrate generate <name> -d crates/db

# Database management via CLI (preferred over sea-orm-cli)
cargo run -p rustvoice -- db status   # show applied/pending migrations
cargo run -p rustvoice -- db up       # apply pending migrations
cargo run -p rustvoice -- db down     # roll back one migration
cargo run -p rustvoice -- db fresh    # drop all tables and reapply (dev reset)

# Re-register slash commands with Discord
cargo run -p rustvoice -- register              # guild-scoped (instant) if DISCORD_SERVER_ID set
cargo run -p rustvoice -- register --global     # force global (up to 1 h propagation)
```

## Architecture

Cargo workspace with four crates under `crates/`:

- **`rustvoice`** (binary) — `clap` CLI. Subcommands: `setup [db]`, `run`,
  `daemon {start,stop,status}`, `db {status,up,down,fresh,refresh,reset}`,
  `register [--global] [--guild <id>]`, `stats`, `cleanup`. No Discord or
  database logic lives here.
- **`bot`** (library) — All Discord interaction. Poise slash commands under
  `commands/`, Serenity event handlers under `events/`. `activity.rs` computes
  activity-based channel names from member presences. `ipc_server.rs` starts a
  Unix socket server so the CLI can query the running bot.
- **`db`** (library) — SeaORM entities (`guilds`, `primary_channels`,
  `temporary_channels`) in `entities/`, thin async wrappers in `repositories/`.
  `connection.rs` builds the pool and runs migrations. `management.rs` exposes
  migration operations (status, up, down, fresh, refresh, reset). `migrator.rs`
  + `migrations/` contain the embedded migration list.
- **`ipc`** (library) — Shared `Request`/`Response` protocol types in
  `protocol.rs`, a Tokio `UnixListener` server helper in `server.rs`, and a
  `UnixStream` client helper in `client.rs`. Used by `bot` (server side) and
  `rustvoice` (client side).

## Key Patterns

**Setup TUI**: `rustvoice setup` launches a `ratatui`/`crossterm` full-screen
form. Navigate with `↑↓` or `Ctrl+P`/`Ctrl+N`, `Enter` to edit a field,
`Ctrl+S` to save. After saving, prompts to apply pending migrations — the DB URL
is read directly from the saved field value, not from the process environment
(which still holds the pre-TUI value).

**Discord library**: Poise 0.6.1 on top of Serenity 0.12.2. Slash commands use
`#[poise::command(slash_command, guild_only)]`. The shared state
`Data { db, start_time }` is in `bot/src/lib.rs`. `Error` and `Context<'a>` type
aliases are also defined there and imported throughout the crate.

**Slash command registration**: On startup, when `DISCORD_SERVER_ID` is set the
bot registers commands in that guild only (instant propagation) and clears all
global commands. Without it, commands are registered globally. Use
`rustvoice register` to force re-registration without restarting the bot.
`bot::client::all_commands()` is the single source of truth for the command list.

**Permission guard**: The `/init` admin command uses a `check` function
(`has_manage_channels`) instead of `required_permissions` so the error message
can be customised. The channel parameter is restricted to voice channels via
`#[channel_types("Voice")]` plus a server-side type check.

**Error handling**: `client::on_error` distinguishes `FrameworkError::Command`
(logs the inner error, sends ephemeral reply to user) from
`FrameworkError::CommandCheckFailed` (sends "no permission" ephemeral). All
command errors are surfaced to the invoking user as ephemeral messages.

**Database**: SeaORM with SQLite. Entity table names are plural (`guilds`,
`primary_channels`, `temporary_channels`) — migration `DeriveIden` enums must
also be plural (e.g. `enum Guilds`, `enum PrimaryChannels`) so their `Table`
variant produces the right name. All database access goes through the repository
functions in `db/src/repositories/`. `db::connection::connect` auto-creates
parent directories and touches the file before opening so SQLite never sees a
missing path. `db::connection::connect_raw` connects without running migrations
(used by `db status` and migration checks in setup).

**Voice channel lifecycle**: All `VoiceStateUpdate` events go through
`events/voice_state.rs`. Join a primary channel → create temp channel + move
user + insert DB row. Leave a temp channel → if empty, delete Discord channel +
delete DB row. `activity::suggested_name` is called on every membership change
and renames the channel if ≥ 50 % of members share a game.

**Daemon**: `rustvoice daemon start` forks before Tokio starts (in `main()`,
not inside `Cli::run()`). This is intentional — forking a multi-threaded Tokio
runtime is unsafe. The child creates a fresh runtime and runs the bot. The PID
file and socket path both resolve via the same priority: `IPC_SOCKET_PATH` env /
`XDG_RUNTIME_DIR` / `~/.local/run` / `/tmp`; see `ipc::default_socket_path()`
and `ipc::default_pid_path()`. `daemon stop` sends SIGTERM by reading the PID
file.

**Startup cleanup**: On reconnect, `events/mod.rs` handles each `GuildCreate`
event (guard `is_new != Some(true)`) and calls `startup_cleanup`. It checks every
`temporary_channel` DB row for that guild: if the Discord channel no longer
exists → remove DB row only; if it exists but is empty (per `guild.voice_states`)
→ delete the channel and the DB row. This handles channels that went empty while
the bot was offline.

**IPC cleanup**: `rustvoice cleanup` → `Request::Cleanup` → `ipc_server::cleanup`.
Requires the bot to be ready (uses `Arc<OnceLock<BotContext>>`). For each
`temporary_channel` row, performs an HTTP `get_channel` check; stale entries are
removed from DB. Existing empty channels are deleted from Discord and DB using the
cache for member counts.

**IPC**: Newline-delimited JSON over a Unix socket (one request line → one
response line). The daemon side (`bot/src/ipc_server.rs`) listens; CLI
subcommands (`daemon status`, `stats`, `cleanup`) connect as clients via
`ipc::client::IpcClient`.

**Logging**: `tracing` + `tracing-subscriber`. Verbosity driven by `-v` count on
the CLI (0 = ERROR … 4 = TRACE).

## Environment

Copy `.env.example` to `.env`:

| Variable            | Description                                                             |
| ------------------- | ----------------------------------------------------------------------- |
| `DISCORD_TOKEN`     | Bot token from Discord Developer Portal                                 |
| `DISCORD_SERVER_ID` | Guild snowflake (used for dev guild-scoped command registration)        |
| `DATABASE_URL`      | `sqlite:./db.sqlite` or an absolute path                                |
| `IPC_SOCKET_PATH`   | Unix socket path; defaults to `$XDG_RUNTIME_DIR/rustvoice.sock` (see `ipc::default_socket_path`) |
