# Completed column: group by task name

## Intent and why

The Completed column should mirror the Pending column's grouping treatment, but only at the task-name level: collapsible name groups with counts, tasks listed flat under each name, no date sub-groups. This supersedes the flat list + "Load more" cap: length is handled by collapsing groups plus the internal scroll both columns already have.

## Goals

- Completed groups by `path[0]` (task name), names in first-appearance order merged across docs, tasks in document order within a name.
- Collapsible groups (default open, chevron + `name (count)`, `aria-expanded`), same interaction as Pending.
- Completed cards drop the `task › …` context line (the group header carries the name).
- One grouped-column component serves both columns.

## Non-goals

- No date sub-groups or date sorting on Completed — everything sits flat under its name.
- No backend change; no change to Pending's grouping, sorting, or tests' behavior.
- No context lines anywhere: `TaskCard`'s context line and `showContext` prop become dead and are deleted.

## Constraints

- `App.test.jsx` 'caps the Completed column at 10 with Load more' must be replaced (Load more is removed); other App tests keep passing.
- `PendingColumn`'s four tests keep passing after the component is generalized (same rendered output, renamed component).
- jsdom: overflow asserted via class names; group labels plain text.

## Approaches

1. **Standalone `CompletedColumn` duplicating the collapse shell** — rejected: duplicates the stateful group logic and card shell (~30 lines) for a second user of the exact same interaction.
2. **Reuse `PendingColumn` via a synthetic labeled subgroup ("No Due Date")** — rejected: renders a misleading date-ish label the user explicitly doesn't want on Completed.
3. **Generalize `PendingColumn` into `GroupedColumn` with a unified groups shape** — picked: `groups: [{ name, subgroups: [{ label: string | null, tasks }] }]`; the label paragraph renders only when `label` is non-null, so Pending renders its date labels unchanged and Completed's single unlabeled subgroup renders bare tasks. Two real users, one component, no duplicated logic.

## Design

- `grouping.js`: add `groupCompleted(tasksResponse) → [{ name, subgroups: [{ label: null, tasks }] }]` (Completed-only tasks); delete `splitCompleted`. No date logic, no ISO sorting — document order throughout.
- `PendingColumn.jsx` → `GroupedColumn.jsx`: rename component, render the subgroup label only when non-null. Props unchanged.
- `TaskCard.jsx`: delete `showContext` and the context line.
- `App.jsx`: both columns render `GroupedColumn`; `board.completed = groupCompleted(…)`; `limit={10}` prop goes.
- Delete `KanbanColumn.jsx` (no consumers left).
- Tests: `grouping.test.js` swaps the `splitCompleted` describe for `groupCompleted`; `board.test.jsx` moves the flat-column tests (markdown/script/markdown-link/line-through, overflow, title count) onto `GroupedColumn` fixtures and deletes the Load-more and context-path tests; `App.test.jsx` replaces the cap test with a grouped assertion (`Completed (12)` with all tasks visible).

## Edge cases

- Completed tasks under a flat header (no nesting): name = that header, consistent with Pending.
- Empty completed side: `Completed (0)`, no groups.
- Same deferred-minor caveat as Pending: a malformed `path: []` payload would key an `undefined` name group (unreachable from the real backend).
