# RustVoice

A Discord bot that creates temporary voice channels on demand and names them after the game the majority of members are playing.

## How it works

1. A server admin runs `/init` and picks a voice channel to act as the trigger.
2. When a member joins that channel, the bot creates a new temporary voice channel in the same category and moves them there.
3. The channel is named after the game most members are playing (`[GameName]`), or `[General]` if there's no majority.
4. The name updates live as members join and leave.
5. When the last member leaves, the channel is automatically deleted.

## Slash commands

| Command | Who | Description |
|---|---|---|
| `/init <channel>` | Admin (Manage Channels) | Register a voice channel as a trigger |
| `/rename <name>` | Anyone in a temp channel | Rename your current channel |
| `/limit <n>` | Anyone in a temp channel | Set a user limit (0 = unlimited) |
| `/unlimit` | Anyone in a temp channel | Remove the user limit |
| `/private` | Anyone in a temp channel | Make the channel invite-only |
| `/public` | Anyone in a temp channel | Make the channel open again |

## Setup

### Option A — Docker (recommended)

**Prerequisites:** Docker with the Compose plugin.

1. Create a bot at the [Discord Developer Portal](https://discord.com/developers/applications), enable the **Server Members Intent** and **Presence Intent**, and copy the token.

2. Create a `compose.yaml` and `.env` in the same directory:

```yaml
# compose.yaml
name: rustvoice

services:
  bot:
    image: ghcr.io/mondotosz/moncorp-rustvoice:latest
    restart: unless-stopped
    environment:
      DISCORD_TOKEN: ${DISCORD_TOKEN}
      DATABASE_URL: sqlite:/data/db.sqlite
      DISCORD_SERVER_ID: ${DISCORD_SERVER_ID:-}
      IPC_SOCKET_PATH: /tmp/rustvoice.sock
    volumes:
      - data:/data
    healthcheck:
      test: ["CMD", "rustvoice", "daemon", "status"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s

volumes:
  data:
```

```sh
# .env
DISCORD_TOKEN=your_bot_token_here
# DISCORD_SERVER_ID=your_guild_id  # optional, enables instant command registration
```

3. Start:

```bash
docker compose up -d
docker compose ps   # should show "healthy" after ~30 s
```

The database is created and migrated automatically on first start. Updating to a new version is just `docker compose pull && docker compose up -d`.

---

### Option B — Build from source

**Prerequisites:** Rust toolchain, `cargo`.

```bash
git clone https://github.com/Mondotosz/moncorp-rustvoice
cd moncorp-rustvoice

# Interactive setup — writes .env and applies migrations
cargo run -p rustvoice -- setup

# Run in foreground
cargo run -p rustvoice -- run

# Or run as a background daemon
cargo run -p rustvoice -- daemon start
cargo run -p rustvoice -- daemon status
cargo run -p rustvoice -- daemon stop
```

#### Environment variables

Copy `.env.example` to `.env` and fill in the values:

| Variable | Required | Description |
|---|---|---|
| `DISCORD_TOKEN` | Yes | Bot token from the Developer Portal |
| `DATABASE_URL` | Yes | SQLite path, e.g. `sqlite:./db.sqlite` |
| `DISCORD_SERVER_ID` | No | Guild snowflake — enables instant slash command registration during development |
| `IPC_SOCKET_PATH` | No | Defaults to `$XDG_RUNTIME_DIR/rustvoice.sock`, then `~/.local/run/`, then `/tmp/` |

#### Database

Migrations run automatically on startup. The CLI also exposes manual controls:

```bash
cargo run -p rustvoice -- db status   # show applied / pending migrations
cargo run -p rustvoice -- db up       # apply pending
cargo run -p rustvoice -- db down     # roll back one
cargo run -p rustvoice -- db fresh    # drop all tables and reapply (dev reset)
```

## Project structure

```
crates/
├── rustvoice/   # Binary — clap CLI, subcommand dispatch, daemon lifecycle
├── bot/         # Library — Poise slash commands, Serenity event handlers, IPC server
├── db/          # Library — SeaORM entities, migrations, repository functions
└── ipc/         # Library — Unix socket protocol shared by bot (server) and CLI (client)
```

- **`rustvoice`** owns the process boundary: it daemonizes, wires up the Tokio runtime, and delegates everything else.
- **`bot`** contains all Discord logic. Commands live in `commands/`, event handlers in `events/`. `activity.rs` computes the suggested channel name. `ipc_server.rs` starts the Unix socket server so the CLI can query the live bot.
- **`db`** is the only crate that touches the database. All access goes through the thin async wrappers in `repositories/` — never use SeaORM `ActiveModel` directly outside this crate.
- **`ipc`** defines the `Request`/`Response` protocol and provides both a server helper (`tokio::net::UnixListener`) and a client helper used by the CLI subcommands.
