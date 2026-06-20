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

# Generate an OAuth2 invite URL with all required bot permissions
cargo run -p rustvoice -- invite
```

## Architecture

Cargo workspace with four crates under `crates/`:

- **`rustvoice`** (binary) — `clap` CLI. Subcommands: `setup [db]`, `run`,
  `daemon {start,stop,status}`, `db {status,up,down,fresh,refresh,reset}`,
  `register [--global] [--guild <id>]`, `stats`, `cleanup`, `invite`. No Discord or
  database logic lives here. Uses `anyhow` for error propagation.
- **`bot`** (library) — All Discord interaction. Poise slash commands under
  `commands/`, Serenity event handlers under `events/`. `activity.rs` computes
  activity-based channel names from member presences. `ipc_server.rs` starts a
  Unix socket server so the CLI can query the running bot. `error.rs` defines
  `BotError` (thiserror enum), the concrete error type used by all commands and
  public APIs. `permissions.rs` is the single source of truth for all permission
  metadata, the `BotPermissionError` type (one variant of `BotError`), and the
  `PermissionResultExt` trait. `leveling.rs` implements the XP formula and helper
  functions (`xp_for_level`, `level_from_xp`, `format_duration`, `progress_bar`).
  `events/xp.rs` handles XP accrual and daily bonus on voice state transitions.
- **`db`** (library) — SeaORM entities (`guilds`, `primary_channels`,
  `temporary_channels`, `user_profiles`, `voice_sessions`) in `entities/`, thin
  async wrappers in `repositories/`. `error.rs` defines `DbError` (thiserror enum
  with `Db(sea_orm::DbErr)` and `Io(std::io::Error)` variants) — all public
  functions return `Result<_, DbError>`. `connection.rs` builds the pool and runs
  migrations. `management.rs` exposes migration operations (status, up, down,
  fresh, refresh, reset). `migrator.rs` + `migrations/` contain the embedded
  migration list. `user_profiles` and `voice_sessions` both use composite primary
  keys `(user_id, guild_id)`.
- **`ipc`** (library) — Shared `Request`/`Response` protocol types in
  `protocol.rs`, a Tokio `UnixListener` server helper in `server.rs`, and a
  `UnixStream` client helper in `client.rs`. `IpcError` (thiserror enum) is the
  typed error for all `IpcClient` operations (`connect` and `send`). Used by
  `bot` (server side) and `rustvoice` (client side).

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

**Permission guard**: The `/init` and `/permissions` admin commands use a `check`
function (`has_manage_channels`) instead of `required_permissions` so the error
message can be customised. The channel parameter is restricted to voice channels via
`#[channel_types("Voice")]` plus a server-side type check.

**Permission system**: `bot/src/permissions.rs` is the single source of truth.
It defines `PermissionEntry` (metadata per permission), `ENTRIES` (display-ordered
slice), `CORE`/`PRIVACY`/`ALL` permission set constants, `BotPermissionError`
(thiserror struct that carries `required: &'static [Permissions]` and
`required_names: String` pre-computed at creation), and the `PermissionResultExt`
trait whose `.requires(&[Permissions::X])` method wraps any
`Result<T, serenity::Error>` at the call site. Every Discord API call that can fail
with 50013 annotates itself this way. `bot::invite_url()` in `lib.rs` uses
`permissions::ALL.bits()` to generate the OAuth2 URL.

**Error handling**: `client::on_error` distinguishes `FrameworkError::Command`
(pattern-matches `BotError::Permission` directly — no downcast needed — computes
which required permissions are actually missing via `bot_guild_permissions`, and
shows a precise ephemeral message; if `MANAGE_ROLES` is missing, adds a note that
it can be granted at the voice category level instead of server-wide) from
`FrameworkError::CommandCheckFailed` (sends "no permission" ephemeral). All
command errors are surfaced to the invoking user as ephemeral messages.
`bot_guild_permissions` tries the Serenity cache first and falls back to an HTTP
member fetch if the bot is not present in the cached guild member map.

**Database**: SeaORM with SQLite. Entity table names are plural (`guilds`,
`primary_channels`, `temporary_channels`) — migration `DeriveIden` enums must
also be plural (e.g. `enum Guilds`, `enum PrimaryChannels`) so their `Table`
variant produces the right name. All database access goes through the repository
functions in `db/src/repositories/`. `db::connection::connect` auto-creates
parent directories and touches the file before opening so SQLite never sees a
missing path. `db::connection::connect_raw` connects without running migrations
(used by `db status` and migration checks in setup).

**XP and leveling**: `bot/src/leveling.rs` defines the XP curve: geometric
progression (BASE=3600s, GROWTH=1.047) for levels 1–100 so level 1→2 costs 1h and
level 100 ≈ 2000h total; arithmetic +24h per level beyond 100. XP is stored in
seconds (1s of voice = 1 XP). A voice session must last at least 60 s before any
XP is awarded. The daily bonus (3600 XP ≈ 1h) is awarded on the first join of a
bot-managed temp channel inside a ±2 h grace window around the 24 h cadence: eligible
at 22 h, in-window up to 26 h. When in-window, `last_daily_at` is advanced by exactly
24 h (not set to `now`), keeping the anchor stable. Missing the window resets the
streak to 1. All daily bonus logic lives in `events/xp.rs::award_daily_bonus_if_eligible`.
`/profile [user]` shows an embed with level, XP progress bar (Unicode `█`/`░`),
total voice time, and a streak counter (`🔥 N` or `—`). The streak displayed is
computed at request time: 0 if `last_daily_at` is more than 26 h ago. `/ranking`
shows the server leaderboard sorted by XP with ◀/▶ button pagination (10 per page,
60 s timeout, buttons auto-disabled on expiry).
`user_profiles(user_id, guild_id)` holds XP, voice seconds, last-daily timestamp,
and streak counter; `voice_sessions(user_id, guild_id)` tracks active session join
times. On bot reconnect, open sessions for users who left while offline receive XP
(capped at 4 h, minimum 60 s); sessions for users still in a temp channel are preserved.

**Voice channel lifecycle**: All `VoiceStateUpdate` events go through
`events/voice_state.rs`. Join a primary channel → create temp channel + move
user + insert DB row. Leave a temp channel → if empty, delete Discord channel +
delete DB row (also deletes the associated `[join ↑]` channel if one exists).
`activity::suggested_name` is called on every membership change and renames the
channel if ≥ 50 % of members share a game.

**Join request flow**: `/private` first creates a member-level overwrite for the bot
on the channel (allowing `VIEW_CHANNEL | CONNECT | MANAGE_CHANNELS | MANAGE_ROLES`)
so that category-level permission grants are preserved and the subsequent
`@everyone CONNECT` deny cannot lock the bot out. It then denies `@everyone CONNECT`
and creates a companion `[join ↑]` voice channel in the same category with an
explicit `@everyone CONNECT allow` (so it stays joinable even under a restricted
category). The join channel's Discord ID is stored in `temporary_channels.join_channel_id`.
When someone joins `[join ↑]`, `on_join` detects it via `find_by_join_channel` and
posts an Allow/Deny button message in the private channel's text-in-voice area.
A `tokio::spawn`-ed task drives a `ComponentInteractionCollector` (120 s timeout);
only members currently in the private channel can respond. Allow moves the
requester in; Deny disconnects them. `/public` deletes the `[join ↑]` channel,
clears the DB field, and removes the `@everyone` deny — the bot's member overwrite
is intentionally kept so it never loses its channel-level permissions.

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
exists → remove DB row only (best-effort delete of the associated `[join ↑]`
channel first); if it exists but is empty (per `guild.voice_states`) → delete
both the `[join ↑]` channel and the temp channel, then remove the DB row. After
processing temp channels, all open `voice_sessions` for that guild are inspected:
sessions whose user is still in a live temp channel are kept intact; sessions for
users who left while the bot was offline award XP capped at 4 h (minimum 60 s) and
are then deleted. This runs unconditionally even when there are no temp channel rows.

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

## Docker

```bash
# Build locally
docker build -t rustvoice .

# Run with compose (reads .env from the same directory)
docker compose up -d

# Check health
docker compose ps
```

**`compose.yaml`** mounts a named volume at `/data` and sets `DATABASE_URL=sqlite:/data/db.sqlite`.
Migrations run automatically on every startup — no separate init step needed.
The healthcheck uses `rustvoice daemon status` which connects to the IPC socket the bot exposes even in foreground mode (`rustvoice run`).

## CI/CD

`.github/workflows/ci.yml` runs on every pull request: `cargo fmt --all -- --check`,
`cargo clippy --workspace -- -D warnings`, and `cargo test --workspace`. The Clippy
and Test jobs install `libsqlite3-dev` because SeaORM links against the system SQLite.
Use `Swatinem/rust-cache@v2` to share Cargo caches across runs.

`.github/workflows/docker.yml` builds and pushes to GHCR on every push to `main` or
a `v*` tag. Tags applied: `latest` (on `main`) and semver tags from the git tag (e.g.
`v0.2.0` → `0.2.0` and `0.2`). Uses `docker/metadata-action@v5` for tag extraction
and `type=gha` BuildKit layer cache. `GITHUB_TOKEN` with `packages: write` is the
only credential required — no manual secrets needed for a personal repo.
