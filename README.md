# storykeep

Relationship memory platform.

## Layout

- `apps/backend` — API server (Rust, tokio + warp, Postgres via sqlx)
- `apps/web` — web frontend (React + TypeScript + Vite, managed with Bun)
- `apps/mobile` — mobile app (Flutter, Android + iOS)
- `flake/` — Nix flake modules (devshell, Postgres, Rust, Flutter, Bun)

## Development

Enter the devshell with `nix develop`, then:

```sh
# database (Postgres on port 5433, data in ./data)
backend-services

# backend (reads DATABASE_URL from the devshell; serves on 127.0.0.1:3000)
cd apps/backend && cargo run      # applies pending migrations on startup
cd apps/backend && cargo test     # schema/constraint tests (needs the database running)

# web
cd apps/web && bun install && bun dev

# mobile
cd apps/mobile && flutter run
```

Run `nix run .#gcroots` to protect the flake inputs and devshell from `nix-store --gc`
(re-run it after `nix flake update`).
