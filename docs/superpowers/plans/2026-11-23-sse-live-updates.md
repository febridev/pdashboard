# SSE Live Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The board updates on its own within one poll interval (~60s) of an Outline edit, without a manual browser refresh.

**Architecture:** A backend tokio task polls Outline every `TASK_POLL_INTERVAL_SECS` (default 60), compares each document's `updatedAt` against a local cache, and on change publishes a fresh `TasksResponse` snapshot over a `tokio::sync::watch` channel. A new SSE endpoint `GET /api/tasks/stream` streams snapshots (initial + on-change) to the browser; the frontend reuses its existing fetch for first paint and swaps in board state on each stream event.

**Tech Stack:** Rust (axum 0.7 built-in SSE, tokio watch; new dep: `async-stream`), React/Vitest (jsdom, EventSource stub), nginx (`proxy_buffering off` for the stream route).

**Spec:** `docs/superpowers/specs/2026-11-23-sse-live-updates-design.md`

## Global Constraints

- Poll interval default **60 seconds**, env var `TASK_POLL_INTERVAL_SECS` (spec decision).
- `GET /api/tasks` response shape and behavior unchanged (spec goal); stream payloads reuse the same `TasksResponse` JSON so the frontend can reuse `parseTasksResponse`.
- Stream auth = existing session cookie via `RequireAuth`, same as `/api/tasks` (spec).
- No Outline-side configuration, no webhooks (spec non-goal).
- Frontend is plain JavaScript, tests via vitest/jsdom, `@` alias for `src/`.
- Backend follows existing patterns: logic in `services/` exported via `lib.rs`, thin handlers in `routes/`, integration tests in `backend/tests/`.
- Naming collision to keep straight: `models::document::DocumentTasks` (Outline task counts) vs `models::task::DocumentTasks` (title + groups). The live-updates code uses the `models::task` one.

## Review Focus

1. **Outline sends `updatedAt` camelCase** — a wrong serde rename silently disables change detection forever (poller never sees a change). Test: Task 1's serde tests.
2. **Malformed stream payload** — a bad event must not crash the board or blank it; keep the last good snapshot. Test: Task 5's malformed-JSON and validation-failure cases.
3. **Idle SSE connection behind proxies** — nginx's default 60s read timeout would kill a quiet stream. Pinned by axum `keep_alive` pings every 15s (Task 4) and the 3-minute-tab-open manual check (Task 7).
4. **Document removed or emptied in Outline** — snapshot must drop it and follow `documents.list` order. Test: Task 2's rebuild tests (removed doc, ordering, empty groups).
5. **Transient Outline API failure mid-poll** — board keeps its last snapshot; no partial or empty publish. Structural: publish happens only on success paths (Task 3); manual: wrong API key → warn logs, stream stays silent.

---

### Task 1: Backend plumbing — `updatedAt` on Document, poll interval config

**Files:**
- Modify: `backend/src/models/document.rs`
- Modify: `backend/src/config.rs`
- Modify: `backend/src/models/response.rs`
- Modify: `.env.example`
- Test: `backend/tests/live_test.rs` (create)

**Interfaces:**
- Produces: `Document { id, title, text, tasks, updated_at: Option<String> }` with `#[serde(rename = "updatedAt")]`; `Config.task_poll_interval_secs: u64`; `TasksResponse` gains `Clone`.

- [ ] **Step 1: Write the failing tests** — create `backend/tests/live_test.rs`:

```rust
use pdashboard_backend::models::document::Document;

#[test]
fn document_parses_camel_case_updated_at() {
    let raw = serde_json::json!({
        "id": "d1", "title": "Tasks", "text": "- [ ] a",
        "updatedAt": "2026-11-23T09:00:00.000Z"
    });
    let doc: Document = serde_json::from_value(raw).unwrap();
    assert_eq!(doc.updated_at.as_deref(), Some("2026-11-23T09:00:00.000Z"));
}

#[test]
fn document_without_updated_at_still_parses() {
    let raw = serde_json::json!({ "id": "d1", "title": "Tasks", "text": "- [ ] a" });
    let doc: Document = serde_json::from_value(raw).unwrap();
    assert!(doc.updated_at.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd backend && cargo test`
Expected: FAIL — compile error, `Document` has no field `updated_at`.

- [ ] **Step 3: Implement**

- `models/document.rs`: add to `Document`:
  `#[serde(rename = "updatedAt")] pub updated_at: Option<String>,`
- `models/response.rs`: `#[derive(Serialize, Clone)]` (later tasks clone snapshots out of the watch channel).
- `config.rs`: add field `pub task_poll_interval_secs: u64`, parsed like `PORT` but with default `"60"` from env `TASK_POLL_INTERVAL_SECS`.
- `.env.example`: after the `OUTLINE_TASK_DOCUMENT_ID` block add:

```
# How often (seconds) the backend checks Outline for task changes.
# Live updates reach the browser via SSE within this window.
TASK_POLL_INTERVAL_SECS=60
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd backend && cargo test`
Expected: PASS, including the pre-existing parser tests.

- [ ] **Step 5: Commit**

```bash
git add backend/tests/live_test.rs backend/src/models/document.rs backend/src/models/response.rs backend/src/config.rs .env.example
git commit -m "feat: parse Outline updatedAt and add poll interval config"
```

---

### Task 2: Change-detection core — cache, snapshot rebuild, event channel

**Files:**
- Create: `backend/src/services/live.rs`
- Modify: `backend/src/services/mod.rs` (add `pub mod live;`)
- Test: `backend/tests/live_test.rs` (append)

**Interfaces:**
- Consumes: `Document.updated_at` (Task 1).
- Produces (all in `crate::services::live`):

```rust
pub struct TaskEvents { /* tokio::sync::watch::Sender<Option<TasksResponse>> */ }
impl TaskEvents {
    pub fn new() -> Self;                                   // watch channel, initial value None
    pub fn publish(&self, snapshot: TasksResponse);         // send_replace(Some(snapshot)) — never fails
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<Option<TasksResponse>>;
}

pub struct CachedDoc { pub updated_at: Option<String>, pub title: String, pub groups: Vec<TaskGroup> }
pub type DocumentCache = HashMap<String, CachedDoc>;

pub fn needs_refetch(cache: &DocumentCache, doc: &Document) -> bool;
pub fn rebuild_snapshot(cache: &mut DocumentCache, listed: &[Document]) -> TasksResponse;
```

- [ ] **Step 1: Write the failing tests** — append to `backend/tests/live_test.rs`:

```rust
use std::collections::HashMap;
use pdashboard_backend::models::document::Document;
use pdashboard_backend::models::task::{Task, TaskGroup, TaskStatus};
use pdashboard_backend::models::response::TasksResponse;
use pdashboard_backend::services::live::{CachedDoc, DocumentCache, TaskEvents, needs_refetch, rebuild_snapshot};

fn doc(id: &str, updated: Option<&str>) -> Document {
    Document {
        id: id.to_string(),
        title: format!("Doc {id}"),
        text: None,
        tasks: Some(pdashboard_backend::models::document::DocumentTasks { completed: 0, total: 1 }),
        updated_at: updated.map(str::to_string),
    }
}

fn group() -> Vec<TaskGroup> {
    vec![TaskGroup { header: "g".into(), path: vec![], tasks: vec![Task { text: "t".into(), status: TaskStatus::Pending }] }]
}

#[test]
fn needs_refetch_is_true_for_unknown_or_changed_and_false_when_unchanged() {
    let mut cache: DocumentCache = HashMap::new();
    assert!(needs_refetch(&cache, &doc("a", Some("1"))));            // not cached yet
    cache.insert("a".into(), CachedDoc { updated_at: Some("1".into()), title: "A".into(), groups: group() });
    assert!(!needs_refetch(&cache, &doc("a", Some("1"))));          // unchanged
    assert!(needs_refetch(&cache, &doc("a", Some("2"))));            // updatedAt moved
}

#[test]
fn rebuild_follows_listed_order_and_drops_removed_docs() {
    let mut cache: DocumentCache = HashMap::new();
    cache.insert("a".into(), CachedDoc { updated_at: Some("1".into()), title: "A".into(), groups: group() });
    cache.insert("b".into(), CachedDoc { updated_at: Some("1".into()), title: "B".into(), groups: group() });
    // listed order is b then a — snapshot must follow the list, not the cache
    let snap = rebuild_snapshot(&mut cache, &[doc("b", Some("1")), doc("a", Some("1"))]);
    let titles: Vec<_> = snap.tasks.iter().map(|d| d.document_title.as_str()).collect();
    assert_eq!(titles, ["B", "A"]);
    // "a" disappears from the list -> dropped from cache and snapshot
    let snap2 = rebuild_snapshot(&mut cache, &[doc("b", Some("1"))]);
    assert_eq!(snap2.tasks.len(), 1);
    assert!(!cache.contains_key("a"));
}

#[test]
fn rebuild_skips_docs_with_empty_groups() {
    let mut cache: DocumentCache = HashMap::new();
    cache.insert("a".into(), CachedDoc { updated_at: Some("1".into()), title: "A".into(), groups: vec![] });
    let snap = rebuild_snapshot(&mut cache, &[doc("a", Some("1"))]);
    assert!(snap.tasks.is_empty());   // mirrors get_tasks' !groups.is_empty() filter
    assert!(cache.contains_key("a")); // cached state kept; snapshot exclusion is enough
}

#[test]
fn task_events_publish_makes_snapshot_visible_to_subscribers() {
    let events = TaskEvents::new();
    let mut rx = events.subscribe();
    assert!(rx.borrow_and_update().is_none());       // no snapshot yet
    events.publish(TasksResponse { tasks: vec![] });
    assert!(rx.borrow_and_update().is_some());       // latest value, even with zero prior receivers
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd backend && cargo test`
Expected: FAIL — compile error, `services::live` does not exist.

- [ ] **Step 3: Implement `backend/src/services/live.rs`**

`TaskEvents` wraps `tokio::sync::watch::channel::<Option<TasksResponse>>(None)`; `publish` uses `send_replace` (works with zero receivers). `needs_refetch`: no cache entry or `updated_at` differs → true. `rebuild_snapshot`: drop cache keys not present in `listed`; then for each doc in `listed` order, push `models::task::DocumentTasks { document_title, groups }` from its cache entry, skipping entries missing from the cache or with empty groups.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd backend && cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/services/live.rs backend/src/services/mod.rs backend/tests/live_test.rs
git commit -m "feat: document cache and snapshot broadcast for live updates"
```

---

### Task 3: Poller task + AppState wiring

**Files:**
- Modify: `backend/src/services/live.rs` (add `spawn_poller`)
- Modify: `backend/src/routes/tasks.rs` (add `pub task_events: Arc<TaskEvents>` to `AppState`)
- Modify: `backend/src/main.rs`

**Interfaces:**
- Consumes: `TaskEvents`, `needs_refetch`, `rebuild_snapshot`, `OutlineClient.get_document/list_documents`, `parse_tasks_from_markdown(&str) -> Vec<TaskGroup>`.
- Produces: `pub fn spawn_poller(outline: OutlineClient, events: Arc<TaskEvents>, interval: Duration)`; `AppState.task_events`.

- [ ] **Step 1: Wire the poller in code**

In `services/live.rs`, add (the loop's structure is the deliverable — both modes share the cache):

```rust
pub fn spawn_poller(outline: OutlineClient, events: Arc<TaskEvents>, interval: std::time::Duration) {
    tokio::spawn(async move {
        let mut cache: DocumentCache = HashMap::new();
        loop {
            if let Err(e) = refresh_once(&outline, &events, &mut cache).await {
                tracing::warn!("live: refresh failed, keeping last snapshot: {}", e);
            }
            tokio::time::sleep(interval).await;
        }
    });
}

async fn refresh_once(outline: &OutlineClient, events: &TaskEvents, cache: &mut DocumentCache) -> Result<(), reqwest::Error> {
    if let Some(doc_id) = outline.config.outline_task_document_id.clone() {
        let doc = outline.get_document(&doc_id).await?;
        if needs_refetch(cache, &doc) {
            let groups = doc.text.as_deref().map(parse_tasks_from_markdown).unwrap_or_default();
            cache.insert(doc.id.clone(), CachedDoc { updated_at: doc.updated_at.clone(), title: doc.title.clone(), groups });
            events.publish(rebuild_snapshot(cache, std::slice::from_ref(&doc)));
            tracing::info!("live: published snapshot for document {}", doc_id);
        }
    } else {
        let docs = outline.list_documents().await?;
        let relevant: Vec<Document> = docs.into_iter()
            .filter(|d| d.tasks.as_ref().map_or(false, |t| t.total > 0)).collect();
        let listed_ids: HashSet<&String> = relevant.iter().map(|d| &d.id).collect();
        let cached_ids: HashSet<&String> = cache.keys().collect();
        if listed_ids != cached_ids || relevant.iter().any(|d| needs_refetch(cache, d)) {
            for d in &relevant {
                if !needs_refetch(cache, d) { continue; }
                match outline.get_document(&d.id).await {
                    Ok(full) => {
                        let groups = full.text.as_deref().map(parse_tasks_from_markdown).unwrap_or_default();
                        cache.insert(full.id.clone(), CachedDoc { updated_at: full.updated_at.clone(), title: full.title.clone(), groups });
                    }
                    Err(e) => tracing::warn!("live: failed to fetch {}: {}", d.id, e), // keep prior cache entry
                }
            }
            events.publish(rebuild_snapshot(cache, &relevant));
            tracing::info!("live: published snapshot ({} documents)", relevant.len());
        }
    }
    Ok(())
}
```

Notes: the first loop iteration runs immediately at startup (warm snapshot before any browser connects); single-doc mode publishes even when parsing yields zero groups (the board clears, matching `get_tasks` returning `tasks: []`); an Outline error path never publishes.

In `routes/tasks.rs` add to `AppState`: `pub task_events: Arc<TaskEvents>`. In `main.rs`: create `let task_events = Arc::new(TaskEvents::new());`, put a clone into `AppState`, and before binding the listener call `services::live::spawn_poller(outline_client.clone(), task_events.clone(), std::time::Duration::from_secs(config.task_poll_interval_secs));` (note: `outline_client` is moved into `AppState` afterwards, so clone it first; `services` is re-exported via `lib.rs` but `main.rs` declares its own `mod services;` — use that module path as the other calls do).

- [ ] **Step 2: Run all backend tests**

Run: `cd backend && cargo test`
Expected: PASS (no behavior change yet; compile proves the wiring).

- [ ] **Step 3: Manual verification with your real `.env`**

Run: `cd backend && cargo run`
Expected: within a second of startup, a `live: published snapshot` log line. Wait 90 seconds with no Outline edits — Expected: **no** second publish log. Tick a checkbox in the Outline task document — Expected: a publish log within ~60s.
Failure mode check: run once with `OUTLINE_API_KEY=invalid cargo run` — Expected: only `live: refresh failed, keeping last snapshot` warnings every interval, no publish, and a connected stream (Task 4) stays silent with its last snapshot.

- [ ] **Step 4: Commit**

```bash
git add backend/src/services/live.rs backend/src/routes/tasks.rs backend/src/main.rs
git commit -m "feat: background poller publishes changed snapshots"
```

---

### Task 4: SSE endpoint `GET /api/tasks/stream`

**Files:**
- Modify: `backend/Cargo.toml` (add `async-stream = "0.3"`)
- Modify: `backend/src/routes/tasks.rs` (handler)
- Modify: `backend/src/main.rs` (route registration)

**Interfaces:**
- Consumes: `AppState.task_events` (Task 3), `RequireAuth`.
- Produces: SSE events named `tasks` whose `data` is a `TasksResponse` JSON — same shape as `/api/tasks`, so the frontend reuses `parseTasksResponse`. First event on connect = current snapshot; then one event per publish. Keep-alive: comment pings every 15s. Response header `x-accel-buffering: no`.

- [ ] **Step 1: Implement the handler**

In `routes/tasks.rs` add (the generator's shape is the deliverable):

```rust
use std::convert::Infallible;
use axum::http::{HeaderName, HeaderValue};
use axum::response::sse::{Event, KeepAlive, Sse};

fn sse_event(snapshot: &TasksResponse) -> Event {
    Event::default().event("tasks").data(serde_json::to_string(snapshot).expect("serialize snapshot"))
}

pub async fn task_stream(
    _auth: RequireAuth,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    let mut rx = state.task_events.subscribe();
    let first = rx.borrow_and_update().clone();
    let stream = async_stream::stream! {
        if let Some(snapshot) = first {
            yield Ok::<Event, Infallible>(sse_event(&snapshot));
        }
        loop {
            if rx.changed().await.is_err() { break; }        // channel gone (shutdown)
            if let Some(snapshot) = rx.borrow_and_update().clone() {
                yield Ok(sse_event(&snapshot));
            }
        }
    };
    let sse = Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("ping"));
    ([(HeaderName::from_static("x-accel-buffering"), HeaderValue::from_static("no"))], sse)
}
```

Register in `main.rs`: `.route("/api/tasks/stream", get(task_stream))` next to `/api/tasks`.

- [ ] **Step 2: Run tests**

Run: `cd backend && cargo test`
Expected: PASS (compile check; route behavior verified manually below).

- [ ] **Step 3: Manual verification**

Unauthenticated: `curl -i http://localhost:3000/api/tasks/stream` → Expected: `401` with `Not authenticated`, no stream.
Authenticated: copy the `session` cookie from a signed-in browser (DevTools → Application → Cookies), then `curl -N -H "Cookie: session=<value>" http://localhost:3000/api/tasks/stream` → Expected: `event: tasks` + one `data:` line immediately, then `: ping` comment lines every 15s. Leave it running >60s: connection stays open. Edit the Outline doc while it runs: a second `event: tasks` arrives within ~60s.

- [ ] **Step 4: Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/src/routes/tasks.rs backend/src/main.rs
git commit -m "feat: SSE endpoint streams task snapshots"
```

---

### Task 5: Frontend stream module

**Files:**
- Create: `frontend/src/lib/stream.js`
- Create: `frontend/src/test/fakeEventSource.js` (shared test helper)
- Test: `frontend/src/lib/stream.test.js` (create)

**Interfaces:**
- Consumes: `parseTasksResponse(raw)` from `@/lib/api`.
- Produces: `subscribeTaskStream(onTasks: (tasks: TasksPayload) => void) => unsubscribe: () => void` — opens `EventSource('/api/tasks/stream')`, listens for `tasks` events, parses each `event.data` with `parseTasksResponse` and calls `onTasks`. Malformed JSON or a validation failure is ignored (keep the last good board; never throw). No `onerror` handler: the browser auto-reconnects transient drops, and a fatal close simply leaves the board static until reload.

- [ ] **Step 1: Write the failing tests**

Create `frontend/src/test/fakeEventSource.js`:

```js
export class FakeEventSource {
  constructor(url) { this.url = url; this.listeners = {}; this.closed = false; FakeEventSource.instances.push(this) }
  addEventListener(type, fn) { (this.listeners[type] ??= []).push(fn) }
  close() { this.closed = true }
  dispatch(type, data) { for (const fn of this.listeners[type] ?? []) fn({ data }) }
}
FakeEventSource.instances = []
export function resetFakeEventSources() { FakeEventSource.instances.length = 0 }
```

Create `frontend/src/lib/stream.test.js`:

```js
import { it, expect, vi, beforeEach, afterEach } from 'vitest'
import { FakeEventSource, resetFakeEventSources } from '@/test/fakeEventSource'
import { subscribeTaskStream } from './stream'

beforeEach(() => { vi.stubGlobal('EventSource', FakeEventSource); resetFakeEventSources() })
afterEach(() => vi.unstubAllGlobals())

const payload = { tasks: [{ document_title: 'T', groups: [{ header: 'H', path: ['H'], tasks: [{ text: 'a', status: 'Pending' }] }] }] }

it('subscribes to /api/tasks/stream and delivers parsed snapshots', () => {
  const seen = []
  subscribeTaskStream((tasks) => seen.push(tasks))
  const es = FakeEventSource.instances.at(-1)
  expect(es.url).toBe('/api/tasks/stream')
  es.dispatch('tasks', JSON.stringify(payload))
  expect(seen).toHaveLength(1)
  expect(seen[0].tasks[0].groups[0].tasks[0].text).toBe('a')
})

it('ignores malformed JSON and invalid payloads without throwing', () => {
  const seen = []
  subscribeTaskStream((tasks) => seen.push(tasks))
  const es = FakeEventSource.instances.at(-1)
  expect(() => es.dispatch('tasks', 'not json')).not.toThrow()
  expect(() => es.dispatch('tasks', JSON.stringify({ tasks: [{ document_title: 5 }] }))).not.toThrow()
  expect(seen).toHaveLength(0)
})

it('closing the unsubscribe function closes the connection', () => {
  const unsubscribe = subscribeTaskStream(() => {})
  const es = FakeEventSource.instances.at(-1)
  unsubscribe()
  expect(es.closed).toBe(true)
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd frontend && npx vitest run src/lib/stream.test.js`
Expected: FAIL — cannot resolve `./stream`.

- [ ] **Step 3: Implement `frontend/src/lib/stream.js`**

Per the Interfaces contract: `new EventSource('/api/tasks/stream')`, `addEventListener('tasks', ...)`, try/catch around `parseTasksResponse(JSON.parse(event.data))`, return `() => source.close()`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd frontend && npx vitest run`
Expected: PASS — stream tests and all pre-existing tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/stream.js frontend/src/lib/stream.test.js frontend/src/test/fakeEventSource.js
git commit -m "feat: subscribe to the task snapshot stream"
```

---

### Task 6: Wire the stream into App

**Files:**
- Modify: `frontend/src/App.jsx`
- Modify: `frontend/src/App.test.jsx`

**Interfaces:**
- Consumes: `subscribeTaskStream` (Task 5), `FakeEventSource` helper (Task 5).

- [ ] **Step 1: Write the failing tests**

In `App.test.jsx`: import the fake, and register a **second** `beforeEach(() => { vi.stubGlobal('EventSource', FakeEventSource); resetFakeEventSources() })` below the existing `beforeEach(() => vi.unstubAllGlobals())` — hooks run in registration order, so the stub must be registered after the unstub hook to survive it. Without this, the new wiring would throw `EventSource is not defined` in every existing test. Add:

```jsx
it('updates the board when the stream pushes new tasks', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(onePending) }))
  render(<App />)
  expect(await screen.findByText('Pending (1)')).toBeInTheDocument()
  FakeEventSource.instances.at(-1).dispatch('tasks', JSON.stringify(twoPending))
  expect(await screen.findByText('Pending (2)')).toBeInTheDocument()
})

it('opens no stream when the fetch is unauthorized', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 401 }))
  render(<App />)
  await screen.findByText('Please sign in to view your tasks')
  expect(FakeEventSource.instances).toHaveLength(0)
})
```

`onePending`/`twoPending` reuse the existing fixture shape with 1 and 2 pending tasks respectively.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd frontend && npx vitest run src/App.test.jsx`
Expected: FAIL — new tests fail (no stream wiring); existing tests fail with `EventSource is not defined` until the `beforeEach` stub is added.

- [ ] **Step 3: Implement in `App.jsx`**

In the existing `useEffect`: after the fetch resolves ok, call `unsubscribe = subscribeTaskStream((tasks) => setState({ status: 'ok', message: '', tasks }))`; keep the `.catch` untouched; return `() => unsubscribe?.()` from the effect so unmount closes the connection. No other component changes.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd frontend && npx vitest run`
Expected: PASS — all App tests, stream tests, and lib tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/App.jsx frontend/src/App.test.jsx
git commit -m "feat: board live-updates from the snapshot stream"
```

---

### Task 7: nginx streaming route, docs, end-to-end check

**Files:**
- Modify: `frontend/nginx.conf`
- Modify: `README.md`

**Interfaces:**
- Consumes: the endpoint from Task 4 behind nginx.

- [ ] **Step 1: Add the stream location to `frontend/nginx.conf`**

Insert before `location /api/` (exact match wins over the prefix):

```nginx
    # SSE: live task snapshots. Unbuffered so events flush as they arrive;
    # long read timeout because the connection is intentionally long-lived
    # (axum sends a keep-alive ping every 15s).
    location = /api/tasks/stream {
        resolver 127.0.0.11 valid=30s ipv6=off;
        set $backend_upstream http://backend:3000;
        proxy_pass $backend_upstream;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 3600s;
    }
```

- [ ] **Step 2: Update `README.md`**

- Features list: add "Live updates — the board refreshes from Outline within the poll interval via SSE".
- `.env` sample: add `TASK_POLL_INTERVAL_SECS=60` with the same comment as `.env.example`.
- Reverse-proxy section: note that the backend sends `x-accel-buffering: no` on the stream (nginx then skips buffering automatically), and recommend the same `proxy_read_timeout`/`proxy_buffering off` lines as the compose nginx for other proxies.
- Local dev note: the Vite dev proxy streams SSE without extra config.

- [ ] **Step 3: End-to-end verification via Docker**

Run: `docker compose up -d --build`
Then in the browser at `http://localhost:8081`: sign in, confirm the board renders. Tick a task checkbox in the Outline document — Expected: the board updates within ~70s **without reloading the page**. Keep the tab open 3+ minutes — Expected: updates still arrive (keep-alive pings keep the connection alive). DevTools Network tab: `/api/tasks/stream` shows EventStream with `tasks` events and ping comments.

- [ ] **Step 4: Commit**

```bash
git add frontend/nginx.conf README.md
git commit -m "feat: stream live updates through nginx and document them"
```
