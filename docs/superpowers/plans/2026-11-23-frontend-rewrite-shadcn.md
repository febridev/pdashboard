# Frontend Rewrite (Dioxus → React + Vite + shadcn/ui) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Dioxus (Rust/WASM) frontend with a plain-JavaScript React + Vite + Tailwind + shadcn/ui SPA that behaves identically, against the unchanged Axum backend.

**Architecture:** Static SPA built by Vite, served by the existing `nginx.conf` (which also proxies `/api` and `/auth` to the backend). Docker goes multi-stage `node:20-alpine` → `nginx:alpine`. Local dev: Vite dev server proxies `/api` and `/auth` to a `cargo run` backend on `localhost:3000`, so full OAuth works at `localhost:5173` (via gitignored `backend/.env` carrying the dev redirect URI).

**Tech Stack:** React, Vite, Tailwind CSS v4, shadcn/ui (Card, Button, Badge, Skeleton), Vitest + @testing-library/react (jsdom).

**Spec:** `docs/superpowers/specs/2026-11-23-frontend-rewrite-shadcn-design.md`

## Global Constraints

- App code is **plain JavaScript (JSX)**; shadcn CLI output may be `.tsx` — esbuild handles it, do not convert.
- **No backend code changes.** API contract is fixed: `GET /api/tasks` → `{"tasks":[{"document_title":str,"groups":[{"header":str,"tasks":[{"text":str,"status":"Pending"|"Completed"}]}]}]}`; 401 = unauthenticated.
- Auth is links only: `/auth/google` (sign in), `/auth/logout` (sign out). No session/token handling in JS.
- `frontend/nginx.conf` stays as-is. `docker-compose.yml` services/ports unchanged (`8081:80`, `3001:3000`); only its line-2 header comment changes (Dioxus → React/Vite).
- Docker image: `node:20-alpine` build stage, `nginx:alpine` runtime, `dist/` → `/usr/share/nginx/html`.
- Behavior quirks to preserve exactly: the "Signed in" status bar renders during the initial load window; a 200 with empty `tasks` renders an empty board (not an error, not sign-in); non-401 failures show an error card, never the sign-in card.
- Cut branch `dioxus-frontend` from `main` before deleting the old files; all work happens on `main`.
- This machine has no Docker and no credentials: Docker cold build + live login are verified by the owner via checklist (Task 8), not by the builder.

## Review Focus

1. **200 with empty tasks** → empty columns, not an error/sign-in card. Test in Task 4 (`splitTasks` empty) and Task 6 (App renders `Pending (0)`).
2. **Malformed response shape or unknown status** (proxy corruption, backend bug) → `ApiError('other')` → error card; today's app fails the same way via serde. Tests in Task 3.
3. **Task text containing HTML/special chars** (`<script>`, quotes) → rendered as literal text. Test in Task 5.
4. **Duplicate identical tasks** (same doc+header+text) → both render, no duplicate-key crash. Keys use flattened index. Test in Task 6.
5. **Network/proxy failure** (fetch rejects) → error card with the message, never "please sign in". Tests in Tasks 3 and 6.

---

### Task 1: Preserve and clear the Dioxus frontend

**Files:**
- Delete: `frontend/Cargo.toml`, `frontend/Cargo.lock`, `frontend/Dioxus.toml`, `frontend/src/`, `frontend/target/`, `frontend/index.html`, `frontend/.gitignore`, `frontend/.dockerignore`
- Keep: `frontend/nginx.conf` (byte-identical)

**Interfaces:**
- Produces: an empty `frontend/` (only `nginx.conf`) for Task 2 to scaffold into; branch `dioxus-frontend` holding the old version.

- [ ] **Step 1: Cut the preservation branch**

```bash
git branch dioxus-frontend
git show dioxus-frontend:frontend/Cargo.toml > /dev/null && echo preserved
```
Expected: `preserved`

- [ ] **Step 2: Delete the Dioxus files (staged)**

```bash
git rm -r --quiet frontend/Cargo.toml frontend/Cargo.lock frontend/Dioxus.toml frontend/src frontend/index.html frontend/.gitignore frontend/.dockerignore
rm -rf frontend/target
```
Expected: `ls frontend/` shows `nginx.conf` and the old `Dockerfile` (Task 7 rewrites it in place).

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: remove Dioxus frontend (preserved on dioxus-frontend branch)"
```

### Task 2: Scaffold Vite + React (JS) + Tailwind + shadcn + Vitest

**Files:**
- Create: `frontend/package.json`, `frontend/vite.config.js`, `frontend/jsconfig.json`, `frontend/index.html`, `frontend/src/main.jsx`, `frontend/src/index.css`, `frontend/src/App.jsx` (placeholder), `frontend/src/test/setup.js`, `frontend/src/App.test.jsx`, `frontend/.gitignore`, `frontend/.dockerignore`, `frontend/components.json` (via shadcn init)

**Interfaces:**
- Produces: npm scripts `dev`/`build`/`test` (`vitest run`); `@` path alias → `frontend/src`; dev proxy `/api` and `/auth` → `http://localhost:3000`; shadcn components `@/components/ui/{button,card,badge,skeleton}`; jsdom test env with `@testing-library/jest-dom/vitest` matchers.

- [ ] **Step 1: Author `package.json` with scripts only, then install**

```bash
cd frontend
# package.json: { "name": "pdashboard-frontend", "private": true, "type": "module", "scripts": { "dev": "vite", "build": "vite build", "preview": "vite preview", "test": "vitest run" } }
npm install react react-dom
npm install -D vite @vitejs/plugin-react vitest jsdom @testing-library/react @testing-library/jest-dom tailwindcss @tailwindcss/vite
```
Expected: installs succeed, lockfile written.

- [ ] **Step 2: Write `vite.config.js`, `jsconfig.json`, `index.html`, `src/main.jsx`, `src/index.css`, `src/test/setup.js`**

`vite.config.js` (exact — proxy, alias, and test env are pinned by the spec):

```js
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath } from 'node:url'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  server: { proxy: { '/api': 'http://localhost:3000', '/auth': 'http://localhost:3000' } },
  test: { environment: 'jsdom', setupFiles: './src/test/setup.js' },
})
```

`jsconfig.json` (shadcn requires the paths alias):

```json
{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["./src/*"] } } }
```

`src/index.css` starts as `@import "tailwindcss";` (shadcn init will extend it).
`src/test/setup.js` is `import '@testing-library/jest-dom/vitest';`.
`index.html`: title `Personal Dashboard`, root `<div id="root">`, module script `/src/main.jsx`.

- [ ] **Step 3: Run shadcn init and add components**

```bash
npx shadcn@latest init --yes --base-color neutral
npx shadcn@latest add button card badge skeleton --yes
```
Expected: `components.json` created, `src/components/ui/` populated (`.jsx` or `.tsx` — either is fine; do not convert). If the CLI prompts or fails on a JS project, write `components.json` manually per the CLI's current docs and retry `add` — the spec leaves CLI details to the builder.

- [ ] **Step 4: Placeholder App + smoke test**

`src/App.jsx` renders `Personal Dashboard` inside shadcn `<Card>`. `src/App.test.jsx`:

```js
import { render, screen } from '@testing-library/react'
import App from './App'

it('renders the dashboard shell', () => {
  render(<App />)
  expect(screen.getByText('Personal Dashboard')).toBeInTheDocument()
})
```

- [ ] **Step 5: Run tests and build**

Run: `npm test` — Expected: 1 passing.
Run: `npm run build` — Expected: `dist/index.html` exists.

- [ ] **Step 6: Write `.gitignore` (node_modules, dist) and `.dockerignore` (node_modules, dist, .git), then commit**

```bash
git add -A && git commit -m "feat: scaffold React+Vite+Tailwind+shadcn frontend with Vitest"
```

### Task 3: API client with response validation

**Files:**
- Create: `frontend/src/lib/api.js`
- Test: `frontend/src/lib/api.test.js`

**Interfaces:**
- Produces:
  - `class ApiError extends Error` with `kind` ∈ `{'unauthorized','other'}` and user-visible `message`
  - `parseTasksResponse(raw: unknown): TasksResponse` — validates the exact contract; throws `ApiError('other', detail)` on any violation
  - `fetchTasks(): Promise<TasksResponse>` — GET `/api/tasks`; 401 → `ApiError('unauthorized', 'Not authenticated')`; other non-OK → `ApiError('other', 'HTTP <status>')`; network reject → `ApiError('other', err.message)`; OK → `parseTasksResponse(res.json())`; OK response whose body fails to parse as JSON → `ApiError('other', 'Invalid JSON response')` (the spec's fidelity rule: parse failures must reach the error card)
  - `TasksResponse = { tasks: Array<{ document_title: string, groups: Array<{ header: string, tasks: Array<{ text: string, status: 'Pending' | 'Completed' }> }> }> }`

- [ ] **Step 1: Write the failing tests**

```js
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ApiError, fetchTasks, parseTasksResponse } from './api'

const fixture = {
  tasks: [
    { document_title: 'Tasks', groups: [
      { header: 'Home', tasks: [
        { text: 'Buy milk', status: 'Pending' },
        { text: 'Pay rent', status: 'Completed' },
      ] },
    ] },
  ],
}

describe('parseTasksResponse', () => {
  it('accepts the API fixture', () => {
    expect(parseTasksResponse(fixture)).toEqual(fixture)
  })
  it('rejects missing tasks', () => {
    expect(() => parseTasksResponse({})).toThrow(ApiError)
  })
  it('rejects a doc without document_title', () => {
    expect(() => parseTasksResponse({ tasks: [{ groups: [] }] })).toThrow(ApiError)
  })
  it('rejects an unknown status', () => {
    expect(() => parseTasksResponse({ tasks: [{ document_title: 'D', groups: [{ header: 'H', tasks: [{ text: 'T', status: 'done' }] }] }] })).toThrow(ApiError)
  })
})

describe('fetchTasks', () => {
  beforeEach(() => vi.unstubAllGlobals())
  it('returns parsed tasks on 200', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(fixture) }))
    await expect(fetchTasks()).resolves.toEqual(fixture)
  })
  it('maps 401 to unauthorized', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 401 }))
    await expect(fetchTasks()).rejects.toMatchObject({ kind: 'unauthorized' })
  })
  it('maps 500 to other with HTTP detail', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500 }))
    await expect(fetchTasks()).rejects.toMatchObject({ kind: 'other', message: 'HTTP 500' })
  })
  it('maps network failure to other with err.message', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('Failed to fetch')))
    await expect(fetchTasks()).rejects.toMatchObject({ kind: 'other', message: 'Failed to fetch' })
  })
})
```

- [ ] **Step 2: Run test to verify it fails** — Run: `npm test` — Expected: FAIL (`Cannot find module './api'`).

- [ ] **Step 3: Implement `api.js`** — `ApiError` with `kind` and `message`; `parseTasksResponse` walking the exact contract (each level checks type; status must be exactly `'Pending'` or `'Completed'`; violation messages name the offending path, e.g. `tasks[0].groups[1].tasks[2].status`); `fetchTasks` per the Interfaces block.

- [ ] **Step 4: Run test to verify it passes** — Run: `npm test` — Expected: PASS (all api tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/api.js src/lib/api.test.js && git commit -m "feat: API client with contract validation and error mapping"
```

### Task 4: Grouping logic

**Files:**
- Create: `frontend/src/lib/grouping.js`
- Test: `frontend/src/lib/grouping.test.js`

**Interfaces:**
- Consumes: `TasksResponse` from Task 3.
- Produces: `splitTasks(tasksResponse): { pending: TaskRef[], completed: TaskRef[] }` with `TaskRef = { docTitle: string, header: string, text: string, status: 'Pending' | 'Completed' }`; order preserved (doc order → group order → task order).

- [ ] **Step 1: Write the failing tests**

```js
import { describe, it, expect } from 'vitest'
import { splitTasks } from './grouping'

// reuse the Task 3 fixture shape, plus a second doc to pin cross-doc order:
// doc 'Tasks' group 'Home' → Buy milk (Pending), Pay rent (Completed)
// doc 'Work' group 'Infra' → Ship release (Pending)

it('splits by status preserving document/group/task order', () => {
  const { pending, completed } = splitTasks(twoDocFixture)
  expect(pending.map((t) => t.text)).toEqual(['Buy milk', 'Ship release'])
  expect(completed.map((t) => t.text)).toEqual(['Pay rent'])
  expect(pending[1]).toEqual({ docTitle: 'Work', header: 'Infra', text: 'Ship release', status: 'Pending' })
})

it('returns two empty arrays when tasks is empty', () => {
  expect(splitTasks({ tasks: [] })).toEqual({ pending: [], completed: [] })
})
```

- [ ] **Step 2: Run test to verify it fails** — Run: `npm test` — Expected: FAIL (`Cannot find module './grouping'`).

- [ ] **Step 3: Implement `splitTasks`** — single pass, three nested loops, pushing a `TaskRef` per task into the array its `status` selects.

- [ ] **Step 4: Run test to verify it passes** — Run: `npm test` — Expected: PASS.

- [ ] **Step 5: Commit** — `git add src/lib/grouping.js src/lib/grouping.test.js && git commit -m "feat: flatten documents into pending/completed columns"`

### Task 5: Board components (KanbanColumn, TaskCard)

**Files:**
- Create: `frontend/src/components/TaskCard.jsx`, `frontend/src/components/KanbanColumn.jsx`
- Test: `frontend/src/components/board.test.jsx`

**Interfaces:**
- Consumes: `TaskRef` from Task 4; shadcn `Card`/`Badge`/`Skeleton` from Task 2.
- Produces:
  - `TaskCard({ taskRef })` — muted small context line `«docTitle» › «header»`; task text; `status === 'Completed'` → `line-through` muted text; pending → amber left accent
  - `KanbanColumn({ title, accent, tasks, loading })` — heading `«title» («tasks.length»)`, e.g. `Pending (2)`; `loading` → 2 `<Skeleton>` placeholders instead of cards

- [ ] **Step 1: Write the failing tests**

```js
import { render, screen } from '@testing-library/react'
import KanbanColumn from './KanbanColumn'

const pending = [
  { docTitle: 'Tasks', header: 'Home', text: 'Buy milk', status: 'Pending' },
  { docTitle: 'Tasks', header: 'Home', text: '<script>alert(1)</script>', status: 'Pending' },
]
const completed = [{ docTitle: 'Tasks', header: 'Home', text: 'Pay rent', status: 'Completed' }]

it('shows the column title with count', () => {
  render(<KanbanColumn title="Pending" accent="amber" tasks={pending} loading={false} />)
  expect(screen.getByText('Pending (2)')).toBeInTheDocument()
})

it('renders task context lines and literal task text', () => {
  render(<KanbanColumn title="Pending" accent="amber" tasks={pending} loading={false} />)
  expect(screen.getByText('Tasks › Home')).toBeInTheDocument()
  expect(screen.getByText('<script>alert(1)</script>')).toBeInTheDocument() // escaped, not executed
})

it('renders completed tasks with line-through styling', () => {
  render(<KanbanColumn title="Completed" accent="green" tasks={completed} loading={false} />)
  expect(screen.getByText('Pay rent')).toHaveClass('line-through')
})

it('renders an empty column with zero count and no cards', () => {
  render(<KanbanColumn title="Pending" accent="amber" tasks={[]} loading={false} />)
  expect(screen.getByText('Pending (0)')).toBeInTheDocument()
})
```

- [ ] **Step 2: Run test to verify it fails** — Run: `npm test` — Expected: FAIL (modules missing).

- [ ] **Step 3: Implement `TaskCard` and `KanbanColumn`** — shadcn `Card` for the column panel and each task card; pending card gets `border-l-4 border-l-amber-400`, completed gets the `line-through` class directly on the text element; loading state uses `Skeleton`. Lists key by flattened index (`tasks.map((taskRef, i) => … key={i}`) — collision-safe by design.

- [ ] **Step 4: Run test to verify it passes** — Run: `npm test` — Expected: PASS.

- [ ] **Step 5: Commit** — `git add src/components && git commit -m "feat: kanban column and task card with shadcn"`

### Task 6: App with auth/error gating

**Files:**
- Modify: `frontend/src/App.jsx` (replace placeholder)
- Modify: `frontend/src/App.test.jsx` (replace smoke test)

**Interfaces:**
- Consumes: `fetchTasks`/`ApiError` (Task 3), `splitTasks` (Task 4), `KanbanColumn`/`TaskCard` (Task 5), shadcn `Button`/`Card`/`Badge` (Task 2).
- Produces: the finished SPA behavior:
  - fetch once on mount; while pending: status bar (`Signed in`) + both columns loading
  - `unauthorized`: centered card — heading `Personal Dashboard`, text `Please sign in to view your tasks`, `Button asChild` linking `href="/auth/google"` labeled `Sign in with Google`
  - `other`: centered card — heading `Failed to load tasks`, the `ApiError.message` text (word-break, never the sign-in card)
  - ok: status bar (`Signed in` + `Sign out` link `href="/auth/logout"`) + `Pending`/`Completed` columns fed by `splitTasks`

- [ ] **Step 1: Write the failing tests**

```js
import { it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import App from './App'

// Stub global fetch per Task 3's pattern (App → real fetchTasks → stubbed
// fetch), so ApiError stays the real class. beforeEach(() => vi.unstubAllGlobals())

it('shows the Signed in bar and loading columns before the fetch resolves', async () => {
  vi.stubGlobal('fetch', vi.fn(() => new Promise(() => {}))) // never resolves
  render(<App />)
  expect(screen.getByText('Signed in')).toBeInTheDocument()
  expect(document.querySelectorAll('[data-slot="skeleton"], .animate-pulse').length).toBeGreaterThan(0)
})

it('shows the sign-in card on unauthorized', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 401 }))
  render(<App />)
  await screen.findByText('Please sign in to view your tasks')
  expect(screen.getByRole('link', { name: 'Sign in with Google' })).toHaveAttribute('href', '/auth/google')
})

it('shows the error card with the message on other failures (never sign-in)', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 502 }))
  await screen.findByText('Failed to load tasks')
  expect(screen.getByText('HTTP 502')).toBeInTheDocument()
  expect(screen.queryByText('Please sign in to view your tasks')).not.toBeInTheDocument()
})

it('renders the board from a 200 with tasks', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(task3Fixture) }))
  expect(await screen.findByText('Pending (1)')).toBeInTheDocument()
  expect(screen.getByText('Completed (1)')).toBeInTheDocument()
  expect(screen.getByRole('link', { name: 'Sign out' })).toHaveAttribute('href', '/auth/logout')
})

it('renders duplicate identical tasks without key collisions', async () => {
  // fixture: the same task text twice under one doc+header → Pending (2)
  expect(await screen.findByText('Pending (2)')).toBeInTheDocument()
})

it('renders an empty board when tasks is empty', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({ tasks: [] }) }))
  expect(await screen.findByText('Pending (0)')).toBeInTheDocument()
  expect(screen.queryByText('Failed to load tasks')).not.toBeInTheDocument()
})
```

- [ ] **Step 2: Run test to verify it fails** — Run: `npm test` — Expected: FAIL (placeholder App has no fetch logic).

- [ ] **Step 3: Implement `App`** — `useState` for `{status: 'loading'|'unauthorized'|'error'|'ok', message, tasks}`; `useEffect` calling `fetchTasks()` once, catching `ApiError` by `kind`; render per the Interfaces block. Exact copy is pinned above; nothing else is user-visible.

- [ ] **Step 4: Run test to verify it passes** — Run: `npm test` — Expected: PASS (full suite).

- [ ] **Step 5: Verify the real backend serves the unauthenticated state**

Run in separate terminals: `cd backend && cargo run` (no `.env` needed for a 401 — hmm, it requires env config at startup; if it fails to start without env vars, note it and skip this manual check — `npm test` covers the state).
Run: `cd frontend && npm run dev`, open `http://localhost:5173`, Expected: sign-in card (proxying `/api/tasks` to :3000 → 401). If the backend won't start without credentials, this step is a no-op covered by tests.

- [ ] **Step 6: Commit** — `git add src/App.jsx src/App.test.jsx && git commit -m "feat: app shell with auth, error, and board states"`

### Task 7: Dockerfile, .dockerignore, compose comment

**Files:**
- Modify: `frontend/Dockerfile` (rewrite), `frontend/.dockerignore` (confirm from Task 2), `docker-compose.yml:2` (comment only)

**Interfaces:**
- Consumes: `npm run build` output `dist/` (Task 2+), `frontend/nginx.conf` (kept from Task 1).
- Produces: a frontend image recipe the owner builds on a Docker-capable host.

- [ ] **Step 1: Rewrite `frontend/Dockerfile` (exact)**

```dockerfile
# Build stage
FROM node:20-alpine AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

# Runtime stage - serve the static bundle and proxy API/auth to the backend.
FROM nginx:alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

- [ ] **Step 2: Fix the compose header comment (line 2)**

`Axum backend + Dioxus frontend served by nginx.` → `Axum backend + React/Vite frontend served by nginx.`

- [ ] **Step 3: Verify the build inputs locally (no Docker on this machine)**

Run: `cd frontend && npm run build` — Expected: `dist/index.html` + `dist/assets/*.js|css` exist; `du -sh dist` shows a few hundred KB.
Run: `grep -n "React/Vite" ../docker-compose.yml` — Expected: line 2 matches.

- [ ] **Step 4: Commit**

```bash
git add frontend/Dockerfile frontend/.dockerignore ../docker-compose.yml
git commit -m "build: node->nginx multi-stage frontend image, fix compose comment"
```

### Task 8: README, env examples, owner verification checklist

**Files:**
- Modify: `README.md` (architecture, dev workflow, verification section), `.env.example:21-22` (replace the stale `cargo run` :3000 comment), `backend/.env.example` (add dev-`backend/.env` note)

**Interfaces:**
- Consumes: everything above; produces the owner-run verification checklist (this machine has no Docker/credentials).

- [ ] **Step 1: Update `.env.example` dev guidance**

Replace lines 21–22 (`For bare cargo run … localhost:3000`) with:
```env
# For local dev (Vite at http://localhost:5173 proxying to cargo run):
# copy the root .env to a gitignored backend/.env and change only this line
# (dotenv loads exactly one file: backend/.env shadows the root, no merge)
# GOOGLE_REDIRECT_URI=http://localhost:5173/auth/google/callback
# Register this URI in the Google console alongside the 8081 one.
```
Same note appended to `backend/.env.example`.

- [ ] **Step 2: Rewrite the README's affected sections**

- Architecture: `Frontend (frontend/): React + Vite + Tailwind + shadcn/ui (JavaScript) — Kanban board UI served by nginx on port 80…` (backend/deployment lines unchanged).
- Local Development — Frontend: `npm install && npm run dev` (replaces `dx serve`), plus: for auth in dev, copy the root `.env` to gitignored `backend/.env` and change only `GOOGLE_REDIRECT_URI` to `http://localhost:5173/auth/google/callback` (dotenv loads exactly one file), register that callback in the Google console, and run `cargo run` in `backend/`.
- Keep verbatim: the HTTPS/Secure-cookie warning, the reverse-proxy section, port guidance.
- Add a short "Verifying the deployment" checklist (this is also handed to the owner at completion): ① Google console has both redirect URIs ② `docker compose up -d --build` succeeds cold ③ `http://localhost:8081` → Google login → board renders with tasks ④ Sign out → sign-in card reappears.

- [ ] **Step 3: Verify no stale references and backend untouched**

Run: `grep -rn -i "dioxus\|dx serve" README.md .env.example docker-compose.yml frontend/` — Expected: no matches (except none at all).
Run: `cd backend && cargo test` — Expected: existing `parser_test` suite passes unchanged.

- [ ] **Step 4: Final local sweep**

Run: `cd frontend && npm test && npm run build` — Expected: full suite green, build succeeds.
Run: `git status` — Expected: clean tree.

- [ ] **Step 5: Commit**

```bash
git add README.md .env.example backend/.env.example
git commit -m "docs: update README and env examples for the React frontend"
```
