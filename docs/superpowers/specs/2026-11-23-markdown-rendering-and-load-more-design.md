# Markdown rendering + Completed "Load more" — Design

Date: 2026-11-23

## Intent

Outline task text contains inline markdown (`**bold**`, `[links](…)`, `` `code` ``,
`~~strike~~`); the dashboard currently shows those symbols raw. Render them
properly, and keep the Completed Kanban column short: first 10 tasks visible, the
rest behind a "Load more" button.

## Goals

- Task cards render inline markdown (bold, italic, links, code, strikethrough).
- Completed column shows the first 10 tasks; "Load more" reveals 10 more per
  click, labeled with how many remain hidden (e.g. "Load more (23)").
- Column header keeps the true total: "Completed (33)".
- Pending column unchanged: all tasks, no limit.

## Non-goals

- No block markdown (headings, lists, tables) inside a card — task text is a
  single line; a paragraph-level render is enough.
- No backend changes — the Rust parser already passes task text through
  verbatim, which is correct for this.
- No pagination/search on Pending.

## Constraints

- XSS safety is non-negotiable: a task containing `<script>alert(1)</script>`
  must never execute. The existing test asserts this and stays (assertion may
  shift from "literal text shown" to "not rendered/executed").
- Frontend is plain JavaScript (no TS), React 19, shadcn/ui, Vitest.

## Approaches

1. **react-markdown (chosen).** Standard React markdown renderer; XSS-safe by
   default (no raw HTML, `urlTransform` sanitizes link protocols). One npm
   install; raw HTML in a task is dropped rather than shown as literal text.
2. **Custom mini-parser (~60 lines, zero deps).** Converts inline markers to
   React nodes and keeps literal-HTML behavior, but we'd own the parser edge
   cases (nested/unbalanced markers) forever.
3. **Strip markdown on the backend.** Rejected — throws away links and bold,
   the opposite of what was asked.

Choice: **react-markdown**, picked with the user. Safety and correctness of a
maintained parser beat avoiding one small dependency on a personal dashboard.

## Design

- Add `react-markdown` to `frontend/package.json`.
- `TaskCard.jsx`: render `task.text` through `<ReactMarkdown>` inside the
  existing text line. Links open in a new tab (`target="_blank"` +
  `rel="noreferrer"`). A few small CSS/Tailwind rules style `strong`, `em`,
  `code`, `a` at card scale; strike-through for completed tasks keeps applying
  to the whole line.
- `KanbanColumn.jsx`: accept an optional `limit` prop. When set and
  `tasks.length > limit`, render `tasks.slice(0, visible)` where `visible`
  starts at `limit` and grows by `limit` per "Load more" click; show a
  ghost-size Button "Load more (N hidden)" while any remain. No limit prop →
  current behavior (Pending).
- `App.jsx`: pass `limit={10}` to the Completed column.
- Update Vitest suites: markdown renders (bold/link/code present as
  elements), script-tag task does not execute, Completed column caps at 10
  and grows on click; existing grouping/api tests untouched.

## Left to the builder

- Exact styling values for inline markdown at card scale.
- Load-more button placement (bottom of the column's card list) and label
  wording, following the sketch above unless it clashes with the UI.
