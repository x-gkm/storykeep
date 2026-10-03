# Storykeep web

The React web client for Storykeep (spec part 9). React 19, TypeScript (strict), Vite, managed with **bun**. It talks to the REST API documented in [`docs/api`](../../docs/api/README.md).

## Setup

Tooling (bun, Postgres client, Rust for the backend) comes from the Nix devshell at the repo root.

```sh
nix develop            # from the repo root
cd apps/web
bun install
bun run dev            # http://localhost:5173
```

Start the backend too (see `apps/backend`); by default the dev server forwards API calls to `http://127.0.0.1:3000`.

## Scripts

| Script | What it does |
|--------|--------------|
| `bun run dev` | Vite dev server with HMR and the API proxy |
| `bun run build` | Type-check (`tsc -b`) and build to `dist/` |
| `bun run preview` | Serve the production build (same proxy as `dev`) |
| `bun run lint` | oxlint |
| `bun run test` | Unit and component tests (vitest + Testing Library, jsdom) |
| `bun run test:watch` | Tests in watch mode |
| `bun run smoke` | End-to-end smoke test over HTTP through a running dev server and real backend (see below) |

## Environment variables

| Variable | Default | Used by |
|----------|---------|---------|
| `API_PROXY_TARGET` | `http://127.0.0.1:3000` | `vite.config.ts`: where `/api` and `/health` are forwarded. Set it in the shell or in `.env.local`. |
| `SMOKE_BASE_URL` | `http://127.0.0.1:5173` | `scripts/e2e-smoke.ts`: the dev server to test against. |

The app calls relative `/api/...` paths only. In development the Vite proxy forwards them, so the backend needs no CORS setup. In production, serve `dist/` from the same origin as the API, or put a reverse proxy in front of both.

### End-to-end smoke test

```sh
# 1. Backend on a scratch database and port
psql -c 'CREATE DATABASE storykeep_web_e2e'
(cd apps/backend && DATABASE_URL=postgresql://storykeep@127.0.0.1:5433/storykeep_web_e2e \
   BIND_ADDR=127.0.0.1:3101 MEDIA_DIR=$(mktemp -d) cargo run)
# 2. Dev server proxied to it
API_PROXY_TARGET=http://127.0.0.1:3101 bun run dev --port 5181
# 3. The smoke test
SMOKE_BASE_URL=http://127.0.0.1:5181 bun run smoke
```

It covers register/login, profile and relationship creation, adding a member, memory creation, multipart upload of an image and a PDF, downloading media through the proxy, timeline filters and pagination, enforcement of the VIEWER role, the full time-capsule lifecycle (sealed media returns 404, then AVAILABLE, open, content visible), measurements and chart series, development records, and logout.

## Architecture

```
src/
  api/          client.ts     fetch/XHR wrapper: token, ApiError, 401 handling, uploads with progress
                types.ts      DTOs mirroring docs/api
                endpoints.ts  one typed function per endpoint (`api.listMemories(...)`, ...)
                queries.ts    TanStack Query keys, shared hooks, query client defaults
  auth/         AuthProvider (session state), RequireAuth / RedirectIfAuthenticated guards
  components/   ui.tsx (form fields, buttons, loading/error/empty states), media.tsx (gallery,
                previews, uploader), LineChart.tsx (SVG chart), TagInput, Countdown, Layout
  lib/          roles.ts (permission helpers), validation.ts (client rules mirroring the API),
                format.ts, errors.ts, files.ts, useObjectUrl.ts
  pages/        one folder per area: relationship/ (timeline, members, settings), memory/,
                capsule/, profile/ (details, development, measurements), plus auth, dashboard, account
  test/         test helpers (fetch mock, app renderer) and component/flow tests
```

- **Routing** (`react-router`): `/login` and `/register`, plus protected routes. `/` lists your relationships grouped by profile. `/relationships/:id` has the timeline, `capsules`, `members` and `settings` tabs. Memories live at `/memories/:id` and `/memories/:id/edit`, capsules at `/capsules/:id`, profiles at `/profiles/new` and `/profiles/:id`, and settings at `/account`. Timeline filters are kept in the URL, so they survive a reload and can be shared as a link.
- **Data fetching** (`@tanstack/react-query`): caching, loading and error states, and invalidation after changes. 4xx errors are not retried.
- **Errors**: every failed call rejects with an `ApiError` carrying `status`, `code` and `message`, taken from the API's `{ "error": { code, message } }` body. Screens show `message`. 403 and 404 get a short explanation.
- **Roles**: `lib/roles.ts` follows the permission table in `docs/api/README.md`. The UI hides or disables actions your role (taken from the relationship or profile DTO) doesn't allow. The server is still the authority, and its 403, 404 and 409 errors are shown inline.
- **Media**: `/api/media/{id}/content` needs the `Authorization` header, so a plain `<img src>` can't load it. `useObjectUrl` downloads the file with the token, displays it from a `blob:` URL, and revokes the URL on unmount. Documents are downloaded the same way when clicked. Uploads use `XMLHttpRequest` with `FormData` parts named `file`, because `fetch` can't report upload progress.
- **Time capsules**: the server never sends sealed content. `CapsuleView` also renders `message` and `media` only when the status is `OPENED`. While a capsule is locked, the page shows a live countdown and refetches when it reaches zero, because `AVAILABLE` is computed by the server when the capsule is read.
- **Charts**: `LineChart` is a small hand-written SVG chart (no chart dependency). There is one chart per measurement type, so different units never share an axis. Hovering shows the nearest point, and the measurements table next to it gives the same data in accessible form.

### Token storage

After login or register, the bearer token is kept in `localStorage` (key `storykeep.token`). It is attached to every request. On any `401 unauthorized` the token is cleared and the user is sent to `/login`, and the page they wanted is remembered for after sign-in.

**Trade-off:** `localStorage` keeps the session across reloads and tabs and is simple. However, any script running on the page can read it, so an XSS bug would expose the token. The safer alternative is an `httpOnly`, `SameSite` session cookie, which scripts can't read. That needs backend support for cookie authentication and CSRF protection, so it is left for later. To limit the risk, the app renders no user-supplied HTML (React escapes all text), and the backend rejects SVG and HTML uploads.

## Dependencies

| Package | Why |
|---------|-----|
| `react-router` | Routing, nested layouts, URL search params for filters |
| `@tanstack/react-query` | Server-state caching, loading/error states, invalidation |
| `vitest`, `@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom`, `jsdom` (dev) | Tests |

`jsdom` is pinned to v26 because newer versions fail under Bun's runtime: vitest runs on Bun here, since the devshell has no Node.
