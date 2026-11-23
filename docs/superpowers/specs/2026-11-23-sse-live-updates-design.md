# SSE Live Updates Design

Date: 2026-11-23

## Intent

When tasks change in Outline (checkboxes ticked, text edited), the dashboard
board updates on its own — no manual browser refresh.

## Why

The board is a live view of an Outline document the owner edits throughout
the day. Today it shows stale data until someone hits reload, which defeats
the purpose of a glanceable dashboard.

## Approach (picked in conversation)

**SSE + backend polling with change detection.**

Options considered:
- Frontend polling — simplest, but every poll re-fetches/re-parses everything
  from Outline and updates lag up to the interval.
- SSE (chosen) — one-way push; backend polls Outline, pushes only on change;
  browser holds one auto-reconnecting connection. Simpler than WS in Axum,
  works with existing cookie auth.
- WebSocket — overkill: bidirectional machinery for a one-way need, manual
  handshake auth, keepalives, extra proxy config.
- Outline webhooks — instant, but requires Outline to reach the backend over
  HTTP (no localhost dev). Future upgrade, not needed for SSE to work.

## How it works

1. **Poller task (backend):** a background tokio task fetches tasks from
   Outline every `TASK_POLL_INTERVAL_SECS` (default 60). It compares each
   document's `updatedAt` against the last snapshot; if nothing changed it
   does nothing, otherwise it re-parses, stores the fresh `TasksResponse`,
   and broadcasts it.
2. **New endpoint `GET /api/tasks/stream` (SSE):** same session-cookie auth
   as `/api/tasks`. On connect it immediately sends the current snapshot
   (so a fresh page gets data without waiting for the next poll tick), then
   pushes a message each time the poller detects a change.
3. **Frontend:** keeps the existing one-shot fetch on mount for initial
   render and error states, then opens `EventSource('/api/tasks/stream')`
   and replaces board state on each message. EventSource auto-reconnects;
   an auth failure surfaces through the existing error handling.
4. **Nginx:** SSE responses must not be buffered — add
   `proxy_buffering off` (and a long `proxy_read_timeout`) for the stream
   route in the frontend nginx config and the README reverse-proxy example.

## Goals

- Board reflects Outline edits within one poll interval (~1 minute), no reload.
- No new auth flows, no Outline-side configuration.
- Existing `/api/tasks` endpoint unchanged (initial load + tests keep working).

## Non-goals

- Webhooks from Outline (possible later for instant updates; SSE design
  leaves room: broadcaster already exists).
- WebSocket / bidirectional channels.
- Multi-user fan-out optimization (single-user dashboard).

## Decisions (mine, correctable)

- Poll interval default 60s (one Outline check per minute is negligible
  traffic), env var `TASK_POLL_INTERVAL_SECS`.
- Change detection via `updatedAt` comparison per document; full
  re-parse only when a doc actually changed.
- Initial page load still uses the existing fetch (loading skeletons and
  unauthorized handling stay as-is); the stream's first event also carries
  a snapshot so the two paths converge immediately after connect.
