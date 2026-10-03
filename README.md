# storykeep

Relationship memory platform.

## Layout

- `apps/web` — web frontend (React + TypeScript + Vite, managed with Bun)
- `apps/mobile` — mobile app (Flutter, Android + iOS)
- `flake/` — Nix flake modules (devshell, Postgres, Rust, Flutter, Bun)

## Development

Enter the devshell with `nix develop`, then:

```sh
# web
cd apps/web && bun install && bun dev

# mobile
cd apps/mobile && flutter run
```

Run `nix run .#gcroots` to protect the flake inputs and devshell from `nix-store --gc`
(re-run it after `nix flake update`).
