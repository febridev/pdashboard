# Frontend Rewrite: Dioxus → React + Vite + Tailwind + shadcn/ui

Date: 2026-11-23
Owner: febridev

## Intent and why

The personal dashboard's frontend is Dioxus (Rust → WASM). Two pains, stated by
the owner:

1. **Docker builds are slow.** The frontend image compiles the Rust toolchain
   and installs `dioxus-cli` inside the container on every build
   (`cargo install dioxus-cli` alone takes several minutes cold), then runs
   `dx build --release` on top of that.
2. **Artifacts are big.** The release WASM alone is 2.4 MB (plus JS glue), and
   `frontend/target` is ~2.1 GB on disk.

The backend is **already Axum** and stays untouched. The rewrite replaces the
UI layer with a JavaScript frontend that supports shadcn/ui, served by the
same nginx proxy pattern, against the same API. Success = same behavior, much
faster and smaller builds.

## Goals

- Replace `frontend/` with **React + Vite + Tailwind CSS + shadcn/ui, written
  in plain JavaScript** (owner's choice of language).
- Identical user-facing behavior to today's app:
  - fetch `GET /api/tasks` once on load;
  - 401 → "please sign in" card with a "Sign in with Google" link to
    `/auth/google`;
  - any other failure → error card with the failure text (never disguised as
    "please sign in");
  - signed in → status bar ("Signed in" + "Sign out" link to `/auth/logout`)
    and a two-column Kanban: **Pending** (count, amber accent cards) and
    **Completed** (count, green accent cards, strikethrough text);
  - each task card shows `documentTitle › header` context plus the task text;
  - loading state while the fetch is in flight.
- Fast, small builds: Docker build well under a minute warm (layer-cached
  `npm ci`), static bundle a few hundred KB raw (~70 KB gzipped), runtime image
  stays `nginx:alpine`.
- **Local dev without Docker**: the Vite dev server proxies `/api` and `/auth`
  to `cargo run` on `localhost:3000`, so the full OAuth flow works at
  `http://localhost:5173`. Two one-time setup pieces make this real (decided
  with the owner): register `http://localhost:5173/auth/google/callback` as a
  second authorized redirect URI in the Google console, and make a
  gitignored `backend/.env` that is a **full copy of the root `.env` with
  only `GOOGLE_REDIRECT_URI` changed** to
  `http://localhost:5173/auth/google/callback` — the backend's dotenv
  loads exactly one `.env`, and `backend/.env` shadows the root file
  entirely (no merge), so dev and Docker each get the right URI with
  nothing to flip between modes. (Today's `dx serve` setup can't do
  any of this; you have to run Docker for auth.)
- `docker-compose.yml` unchanged: same services, ports (frontend `8081:80`,
  backend `3001:3000`), same `GOOGLE_REDIRECT_URI` guidance. Only the frontend
  image builds differently.

## Non-goals

- No backend changes — API shape, cookie names, session/CSRF handling, and
  auth flow all stay exactly as they are.
- No new dashboard features (no filters, no drag-and-drop, no editing). This
  is a swap of the UI layer, not a redesign of the product.
- No visual redesign beyond shadcn's default look. The layout mirrors the
  current two-column board.

## Anti-goals

- Breaking the OAuth flow. Session cookies are `Secure`; per the current
  README, login does not work over plain HTTP on a real host — that warning
  must survive into the updated docs.
- A UI that shows "please sign in" on a network/proxy error — the current app
  deliberately separates 401 from other errors; the rewrite must keep that.
- A result that builds slower or bigger than today (that would defeat the
  entire why).

## Constraints

- **Backend API is fixed.** `GET /api/tasks` returns
  `{"tasks": [{"document_title": str, "groups": [{"header": str, "tasks":
  [{"text": str, "status": "Pending" | "Completed"}]}]}]}`. 401 when no valid
  `session` cookie.
- **Auth is full-page redirects, not AJAX.** `/auth/google` (GET) starts the
  flow; `/auth/google/callback` completes it and sets the httpOnly `session`
  cookie (7 days); `/auth/logout` (GET) clears it. The frontend only renders
  links to these — no token or session handling in JavaScript.
- **nginx keeps proxying** `/api/` and `/auth/` to `backend:3000` with the
  resolver trick for late backend starts (reuse the current `nginx.conf`,
  including the SPA `try_files` fallback).
- Session cookie is `Secure` → OAuth needs HTTPS on a real host (or localhost,
  which browsers treat as a secure context).

## Approaches

Picked in conversation with the owner:

- **A. React + Vite + Tailwind + shadcn/ui (picked).** The official shadcn/ui
  path — full component set, first-class docs and examples. Docker becomes
  `node:20-alpine` build → copy `dist/` into the same `nginx:alpine` runtime.
  Costs: an npm `package.json`/lockfile, React ~45 KB gzipped in the bundle.
- **B. Svelte or Vue + unofficial shadcn port.** Smaller runtime, but the
  ports have fewer components and lag upstream; still a full framework swap.
- **C. Vanilla JS + Tailwind, no component library.** Smallest bundle, but no
  real shadcn — every component hand-built. Works against the stated want.

Why A: the owner explicitly asked for shadcn support; A is the only official
shadcn path, and it hits both stated pains (build speed, artifact size) hard
enough that the small React runtime cost is irrelevant.

## The how (decided with the owner)

- **Language**: plain JavaScript (JSX) per the owner's request, not
  TypeScript.
- **Scaffold**: Vite React template, Tailwind CSS (v4, current shadcn
  default), `shadcn` CLI init. Components used from shadcn: Card, Button,
  Badge, Skeleton — enough for a two-column board.
- **Structure**: single-page app, no router. Roughly `App.jsx` (state +
  fetch + auth/error gating), `KanbanColumn.jsx`, `TaskCard.jsx`, an `api.js`
  fetch wrapper, mirroring today's `KanbanBoard` component.
- **Dockerfile**: multi-stage — `node:20-alpine` (`npm ci`, `npm run build`)
  → `nginx:alpine` + existing `nginx.conf`, `COPY --from=builder ... /dist`
  into `/usr/share/nginx/html`.
- **Behavior fidelity**: match today's app exactly, including its quirks —
  the "Signed in" bar shows during the initial load window, and an
  Outline-side failure yields an empty board rather than an error card.
- **Dev auth setup**: both redirect URIs registered in the Google console
  (5173 for dev plus the compose/production URI); root `.env` keeps the
  Docker URI; gitignored `backend/.env` — a full copy of the root file with
  only the redirect URI changed — serves `cargo run` at 5173 (dotenv loads
  exactly one file: it shadows, not merges). Document both in
  `.env.example` comments and the README.
- **docker-compose.yml** stays as-is except its header comment, which still
  says "Dioxus frontend" — one-word fix.
- **In place, not beside**: cut a `dioxus-frontend` branch first to preserve
  the old version, then delete the Dioxus files (`Cargo.toml`, `Cargo.lock`,
  `Dioxus.toml`, `src/*.rs`, `target/`) and make `frontend/` the JS project.
- **README updates**: architecture section (React/Vite, not Dioxus), dev
  instructions (`npm run dev` + `cargo run` instead of `dx serve`), keep the
  HTTPS/Secure-cookie warnings and reverse-proxy guidance.

## Testing

- Backend untouched — existing `backend/tests/parser_test.rs` must keep
  passing.
- Frontend: light Vitest coverage — the grouping logic that flattens
  `documents → groups → tasks` into Pending/Completed columns, and one render
  test of the board against a fixture of the API JSON. No E2E suite; the app
  is a thin fetch-and-render over a fixed API.

## What's left to the builder

- Exact Tailwind/shadcn CLI versions and init flags current at build time.
  Note: if the shadcn CLI insists on TypeScript for some component, Vite's
  esbuild handles `.tsx` transparently — app code stays JS, this is a builder
  call within the "JavaScript app" envelope.
- Component decomposition and minor copy/styling details within shadcn's
  defaults, as long as behavior and information shown match today's app.
- Verify locally what this machine allows: Vitest green, the dev server
  (with real `cargo run` backend) shows the unauthenticated/loading states,
  `npm run build` produces a working static bundle, backend `cargo test`
  still green. **This machine has no Docker and no credentials** (decided
  with the owner), so the Docker cold build and the signed-in board are
  verified by the owner via a short checklist — compose up → browse 8081 →
  Google login → board renders → sign out — which the builder writes as part
  of the README handover.
