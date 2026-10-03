# storykeep

A relationship memory platform: a private, chronological space for the memories, milestones, media and development records of a special relationship — a child, a pet, a friend, a partner.

See the [V1 specification](docs/spec.md) for the full concept and the [REST API reference](docs/api/README.md) for every endpoint.

## Layout

| Path | What |
|------|------|
| `apps/backend` | REST API server — Rust, tokio + warp, PostgreSQL via sqlx (with versioned migrations) |
| `apps/web` | Web frontend — React + TypeScript + Vite, managed with Bun ([README](apps/web/README.md)) |
| `apps/mobile` | Mobile app — Flutter, Android + iOS ([README](apps/mobile/README.md)) |
| `docs/` | [Specification](docs/spec.md) and [API reference](docs/api/README.md) |
| `flake/` | Nix flake modules: devshell, Postgres, Rust, Flutter + Android SDK, Bun |

## Getting started

Everything (Rust, Bun, Flutter, Android SDK, Postgres, sqlx-cli) comes from the Nix devshell.

```sh
nix develop
```

1. **Start Postgres** (port 5433, data in `./data`, user/database `storykeep`):

   ```sh
   backend-services            # add --tui=false to run without the TUI
   ```

2. **Start the backend** in another devshell. It applies pending migrations on startup and serves on `http://127.0.0.1:3000`:

   ```sh
   cd apps/backend && cargo run
   ```

3. **Start the web app** in another devshell — `http://localhost:5173`, proxying `/api` to the backend:

   ```sh
   cd apps/web && bun install && bun run dev
   ```

4. **Run the mobile app** — as a Linux desktop app (no phone needed):

   ```sh
   cd apps/mobile && flutter run -d linux
   ```

   or on an Android phone over USB, forwarding its port 3000 to this machine:

   ```sh
   adb reverse tcp:3000 tcp:3000
   cd apps/mobile && flutter run --dart-define=API_BASE_URL=http://127.0.0.1:3000
   ```

   See the [mobile README](apps/mobile/README.md) for emulators and other setups.

Register an account in either client, create a profile, and start adding memories.

## Configuration

| Variable | Used by | Default | Purpose |
|----------|---------|---------|---------|
| `DATABASE_URL` | backend, tests, sqlx-cli | set by the devshell | Postgres connection |
| `BIND_ADDR` | backend | `127.0.0.1:3000` | Listen address |
| `MEDIA_DIR` | backend | `media` (relative to the working directory) | Where uploaded files are stored |
| `RUST_LOG` | backend | `info` | Log filter |
| `API_PROXY_TARGET` | web dev server | `http://127.0.0.1:3000` | Backend the `/api` proxy forwards to |
| `API_BASE_URL` | mobile (`--dart-define`) | see the mobile README | Backend base URL |

No secrets live in the repository; the devshell's database user has no password and only listens on localhost.

## Testing

```sh
cd apps/backend && cargo test                   # API, authorization, constraint, upload and capsule-lock tests (needs Postgres running)
cd apps/web && bun run lint && bun run test     # web unit/component tests
cd apps/mobile && flutter analyze && flutter test
```

Backend tests use `#[sqlx::test]`, which creates a throwaway database per test with all migrations applied. `cd apps/web && bun run smoke` runs an end-to-end check through the web dev server against a running backend (see the web README).

## How the V1 acceptance criteria are covered

| Criterion (spec §20) | Backend tests |
|----------------------|---------------|
| Register and log in | `tests/auth.rs` |
| Create a profile, and a relationship involving it | `tests/profiles_relationships.rs` |
| Authorized members can view the timeline | `tests/memories.rs` |
| Create, edit and delete memories; tags and categories | `tests/memories.rs` |
| A memory can contain multiple media files | `tests/media.rs` |
| Child development records and measurements; chart data | `tests/development.rs` |
| Time capsules with a future unlock date; locked content inaccessible | `tests/capsules.rs`, `tests/media.rs` |
| Unauthorized users can't access private data | every API test file (stranger → 404, viewer → 403) |
| Database normalized to 3NF | `migrations/`, `tests/schema.rs` |
| Documented setup procedure | this README |

## Housekeeping

Run `nix run .#gcroots` to protect the flake inputs and devshell from `nix-store --gc` (re-run it after `nix flake update`).
