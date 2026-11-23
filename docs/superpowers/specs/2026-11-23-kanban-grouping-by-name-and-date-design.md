# Kanban: group Pending by task name → date, scrollable columns

## Intent and why

The single Outline task doc (`task`) is written as `## <task-name>` with `## <date>` sub-headings:

```markdown
## smthk-amdb
- [ ] do W

### 2026-11-23
- [ ] do X
- [ ] do Y

### 2026-11-25
- [ ] do Z
```

Today the Rust parser keeps only the *last* heading, so the board shows `task › 2026-11-23` and the task name `smthk-amdb` is lost entirely. The user wants the Pending column to read as: **task name → date sub-group → tasks**, with undated tasks under **No Due Date**. Both columns should scroll internally instead of stretching the page.

## Goals

- Pending column: group by task name (e.g. `smthk-amdb`), each group collapsible with a count.
- Inside a name group: sub-group by the date heading; tasks directly under the name heading → **No Due Date**.
- ISO-dated sub-groups sort chronologically; non-date sub-groups keep appearance order; No Due Date last.
- Pending and Completed columns scroll internally (max-height + `overflow-y-auto`).
- Backend preserves the heading path so the name reaches the frontend.

## Non-goals

- No grouping for the Completed column (stays flat with Load more).
- No parsing of date formats written inside task text.
- No drag-and-drop, no persistence of collapse state.
- No change to which documents are fetched.

## Constraints

- API contract is strictly validated in `frontend/src/lib/api.js`; adding a field must update both sides.
- Existing tests must keep passing: `parser_test.rs` asserts `header == "TODO"` for flat docs, so `header` must stay the immediate heading.
- XSS handling stays as-is (react-markdown); group labels are plain text.

## Approaches

1. **String path in `header`** — parser joins the heading stack into `"smthk-amdb > 2026-11-23"`; frontend splits on `" > "`. No model change, but `" > "` in a real heading becomes ambiguous and existing header expectations change. **Rejected.**
2. **Structured `path` array on each group** — parser tracks heading levels with a stack; `TaskGroup` gains `path: Vec<String>` (root → leaf, including the immediate heading); `header` stays the immediate heading so current tests and card context keep working. Frontend groups off `path` directly. **Picked: explicit structure, no string parsing, backwards compatible.**

## Design

### Backend (`parser.rs`, `models/task.rs`)

- Maintain a stack of `(level, text)` for headings. On a heading of level N: pop entries with `level >= N`, push the new one.
- Each group's `path` = current stack texts in order; `header` = last stack entry (unchanged semantics).
- Groups still only emit when they contain tasks (existing behavior).

### API contract

`groups[i]` gains `path: string[]` (same length order as stack, always ≥ 1 entries). `api.js` validates it as an array of strings.

### Frontend

- `TaskRef` gains `path: string[]`.
- `grouping.js`: `groupPending(response)` → `[{ name, subgroups: [{ label, tasks }] }]`; name = `path[0]`, sub-group label = `path.slice(1).join(' › ')` or `No Due Date`. Names/sub-groups keyed by first appearance; ISO-dated labels sorted ascending.
- `KanbanColumn` renders the hierarchy for Pending; groups collapsible (default expanded, chevron + name + count); task cards drop the `doc › header` context line in Pending (group headers carry it).
- Completed: unchanged flat list + Load more; card context line shows the full path joined with ` › ` (restores the name there too).
- Both columns: `max-h-[70vh] overflow-y-auto` on the content area.

## Edge cases

- Flat docs (`# Project Alpha` → `## TODO`): `path = ["Project Alpha", "TODO"]`, so Pending groups by `Project Alpha`. Only the single configured `task` doc is displayed in practice, so acceptable.
- Duplicate names / dates in separate sections merge into the same group/sub-group.
- Heading-level jumps (`##` → `####`) just stack; no special handling.

## Test plan

- Rust: nested headings produce expected `path` + `header`; same-level heading pops the stack.
- `api.test.js`: `path` validated, missing `path` rejected.
- `grouping.test.js`: name/date/No-Due-Date hierarchy, date sorting, appearance order.
- `board.test.jsx`: group headers with counts, No Due Date label, collapse toggling, overflow class present on both columns, Load more still working.
