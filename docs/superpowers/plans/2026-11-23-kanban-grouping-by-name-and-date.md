# Kanban Name→Date Grouping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Group the Pending column by task name → date sub-groups (No Due Date last), restore the task name the parser currently loses on sub-headings, and make both Kanban columns scroll internally.

**Architecture:** The Rust parser gains a heading-level stack so each group carries `path: string[]` (root→leaf, including the immediate heading); `header` stays the immediate heading for back-compat. `api.js` validates the new field. `grouping.js` adds `groupPending` (name→subgroup hierarchy, ISO-dated subgroups sorted ascending, No Due Date last) and `splitCompleted` (flat list). A new `PendingColumn` component renders the hierarchy with collapsible groups; `KanbanColumn` stays the flat (Completed) column; both get `max-h-[70vh] overflow-y-auto`.

**Tech Stack:** Rust/Axum backend (hand-rolled parser, no new crates), React 19 + Tailwind v4 + vitest + @testing-library/react frontend.

**Spec:** `docs/superpowers/specs/2026-11-23-kanban-grouping-by-name-and-date-design.md`

## Global Constraints

- No new dependencies on either side; the parser stack is hand-rolled.
- API contract is strictly validated in `frontend/src/lib/api.js` — every new field (`path`) gets a validator there.
- Group labels are plain strings rendered as text, never through markdown/HTML (XSS posture unchanged from spec).
- jsdom does no layout: overflow is asserted via class names on `[data-slot="card-content"]`, never computed styles.
- Existing behavior pinned by tests must keep passing: `parser_test.rs` `header` assertions, `board.test.jsx` flat-column tests (fixtures gain `path`, expected strings unchanged where noted).
- Completed column keeps its flat list + Load more; only overflow is added.
- Commit only the files each task names.
- Run backend commands from `backend/`, frontend commands from `frontend/`.

## Review Focus

1. **A same-level heading replaces, not nests, the previous name** — `## A / tasks / ## B`: B's groups must have `path == ["B"]`, never `["A","B"]`. → Task 1, `test_nested_headings_build_path` asserts `groups[2].path == ["other"]`.
2. **Tasks sitting directly under a name heading (no date sub-heading yet) must survive as their own group** and land in No Due Date on the frontend — not be swallowed or dropped. → Task 1 `test_tasks_directly_under_name_heading_keep_short_path`; Task 3 `groups pending by name…` asserts the `No Due Date` subgroup holds `do W`.
3. **A task-name heading repeated later in the doc merges into the same name group** with sub-groups in first-appearance order. → Task 3 `merges duplicate name sections…`.
4. **Sub-group labels that aren't ISO dates** ("reminder", "Nov 23", typo'd dates) must not crash or get sorted as dates — appearance order after the dated ones. → Task 3 `keeps non-ISO subgroup labels…`.
5. **An API response missing `path` must be rejected loudly** (ApiError → error card), not silently rendered as a blank board. → Task 2 `rejects a group without path` and `rejects a non-string path element`.

---

### Task 1: Heading-path stack in the parser

**Files:**
- Modify: `backend/src/models/task.rs:15-19`
- Modify: `backend/src/services/parser.rs`
- Test: `backend/tests/parser_test.rs`

**Interfaces:**
- Consumes: markdown text from Outline documents (unchanged fetch flow).
- Produces: `TaskGroup { header: String, path: Vec<String>, tasks: Vec<Task> }` serialized as `{"header", "path": [...], "tasks"}` — `path` is the heading stack root→leaf **including** the immediate heading; always ≥ 1 entry when a group is emitted; `header` unchanged (immediate heading).

- [ ] **Step 1: Write the failing tests**

Extend the existing `test_parse_tasks_with_headers` with path assertions, then add two tests to `backend/tests/parser_test.rs`:

```rust
#[test]
fn test_nested_headings_build_path() {
    let markdown = "## smthk-amdb\n\n### 2026-11-23\n- [ ] do X\n\n### 2026-11-25\n- [ ] do Y\n\n## other\n- [ ] do Z\n";
    let groups = parse_tasks_from_markdown(markdown);
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].header, "2026-11-23");
    assert_eq!(groups[0].path, vec!["smthk-amdb".to_string(), "2026-11-23".to_string()]);
    assert_eq!(groups[1].header, "2026-11-25");
    assert_eq!(groups[1].path, vec!["smthk-amdb".to_string(), "2026-11-25".to_string()]);
    assert_eq!(groups[2].header, "other");
    assert_eq!(groups[2].path, vec!["other".to_string()]); // same level popped smthk-amdb — no nesting
}

#[test]
fn test_tasks_directly_under_name_heading_keep_short_path() {
    let markdown = "## smthk-amdb\n- [ ] do W\n\n### 2026-11-23\n- [ ] do X\n";
    let groups = parse_tasks_from_markdown(markdown);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].header, "smthk-amdb");
    assert_eq!(groups[0].path, vec!["smthk-amdb".to_string()]);
    assert_eq!(groups[1].path, vec!["smthk-amdb".to_string(), "2026-11-23".to_string()]);
}
```

In the existing test, also pin the flat-doc path:

```rust
assert_eq!(groups[0].path, vec!["Project Alpha".to_string(), "TODO".to_string()]);
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test` (from `backend/`)
Expected: compile error — `no field named 'path'` on `TaskGroup` (tests reference it before the struct has it). Existing tests still compile only after the struct gains the field.

- [ ] **Step 3: Implement the heading stack**

In `models/task.rs`, add to `TaskGroup` (after `header`):

```rust
pub path: Vec<String>,
```

In `services/parser.rs`, replace `current_header: String` with `heading_stack: Vec<(usize, String)>`. Heading handler — the pop/push semantics are the point:

```rust
// count leading '#' as level, rest is the heading text
let level = trimmed.chars().take_while(|c| *c == '#').count();
let text = trimmed.trim_start_matches('#').trim().to_string();
// pop same-or-shallower headings, then push
heading_stack.retain(|(lvl, _)| *lvl < level);
heading_stack.push((level, text));
```

Emit a group (on heading change and at end of input) only when `!current_tasks.is_empty() && !heading_stack.is_empty()`, with `header: heading_stack.last().unwrap().1.clone()` and `path: heading_stack.iter().map(|(_, t)| t.clone()).collect()`. Everything else (task-prefix stripping, status detection, flush-on-heading, flush-at-end) is unchanged.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test` (from `backend/`)
Expected: PASS — 3 tests in `parser_test.rs` (existing one now with path assertions).

- [ ] **Step 5: Commit**

```bash
git add backend/src/models/task.rs backend/src/services/parser.rs backend/tests/parser_test.rs
git commit -m "feat: parser tracks heading path for task groups"
```

---

### Task 2: `path` in the API contract validation

**Files:**
- Modify: `frontend/src/lib/api.js:45-58` (group mapping)
- Test: `frontend/src/lib/api.test.js`

**Interfaces:**
- Consumes: API JSON `groups[i].path: string[]` (Task 1's serialization).
- Produces: `parseTasksResponse(raw)` returns groups as `{ header: string, path: string[], tasks: [...] }`; a group missing `path` or with a non-string element throws `ApiError('other', …)`.

- [ ] **Step 1: Write the failing tests**

Update the shared `fixture` at the top of `api.test.js` to include `path` in the group:

```js
{ header: 'Home', path: ['Home'], tasks: [ /* unchanged */ ] },
```

Add to the `parseTasksResponse` describe:

```js
it('rejects a group without path', () => {
  expect(() => parseTasksResponse({ tasks: [{ document_title: 'D', groups: [{ header: 'H', tasks: [] }] }] })).toThrow(ApiError)
})
it('rejects a non-string path element', () => {
  const bad = { tasks: [{ document_title: 'D', groups: [{ header: 'H', path: ['ok', 7], tasks: [] }] }] }
  expect(() => parseTasksResponse(bad)).toThrow(ApiError)
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test` (from `frontend/`)
Expected: FAIL — the two new rejection tests fail (parser currently ignores `path` entirely, so no throw); `accepts the API fixture` passes again once the fixture carries `path`.

- [ ] **Step 3: Implement**

In `api.js`, inside the group mapping add (reusing existing `checkArray`/`checkString`):

```js
path: checkArray(group.path, `${groupPath}.path`).map((p, l) => checkString(p, `${groupPath}.path[${l}]`)),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test` (from `frontend/`)
Expected: PASS — all api tests; grouping/board tests unaffected (they don't go through `parseTasksResponse`).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/api.js frontend/src/lib/api.test.js
git commit -m "feat: validate group heading path in tasks contract"
```

---

### Task 3: `groupPending` and `splitCompleted`

**Files:**
- Modify: `frontend/src/lib/grouping.js` (add both functions; `splitTasks` stays until Task 4)
- Test: `frontend/src/lib/grouping.test.js` (add describes; `splitTasks` describe stays until Task 4)

**Interfaces:**
- Consumes: validated `TasksResponse` (groups carry `path` from Task 2).
- Produces:
  - `groupPending(tasksResponse) → NameGroup[]` where `NameGroup = { name: string, subgroups: SubGroup[] }`, `SubGroup = { label: string, tasks: TaskRef[] }`.
  - `splitCompleted(tasksResponse) → TaskRef[]` — flat, document→group→task order.
  - `TaskRef = { docTitle: string, header: string, text: string, status: 'Pending' | 'Completed', path: string[] }`.
  - Rules: name = `path[0]`; sub-group label = `path.length > 1 ? path.slice(1).join(' › ') : 'No Due Date'`. Name groups keyed by first appearance (merged across docs by name string); within a group: ISO-dated labels (`/^\d{4}-\d{2}-\d{2}$/`) ascending first, then non-ISO labels in appearance order, then `No Due Date` last. `splitTasks` and its tests are untouched here.

- [ ] **Step 1: Write the failing tests**

Add to `grouping.test.js` a fixture in the user's doc shape and three describes:

```js
const nestedFixture = {
  tasks: [
    {
      document_title: 'task',
      groups: [
        { header: 'smthk-amdb', path: ['smthk-amdb'], tasks: [{ text: 'do W', status: 'Pending' }] },
        { header: '2026-11-25', path: ['smthk-amdb', '2026-11-25'], tasks: [{ text: 'do Z', status: 'Pending' }] },
        { header: '2026-11-23', path: ['smthk-amdb', '2026-11-23'], tasks: [
          { text: 'do X', status: 'Pending' },
          { text: 'do Y', status: 'Completed' },
        ] },
        { header: 'other', path: ['other'], tasks: [{ text: 'do O', status: 'Pending' }] },
      ],
    },
  ],
}
```

```js
describe('groupPending', () => {
  it('groups pending by name, sorts dated subgroups chronologically, No Due Date last', () => {
    const groups = groupPending(nestedFixture)
    expect(groups.map((g) => g.name)).toEqual(['smthk-amdb', 'other'])
    const smb = groups[0]
    expect(smb.subgroups.map((s) => s.label)).toEqual(['2026-11-23', '2026-11-25', 'No Due Date'])
    expect(smb.subgroups[0].tasks.map((t) => t.text)).toEqual(['do X']) // completed do Y excluded
    expect(smb.subgroups[2].tasks.map((t) => t.text)).toEqual(['do W']) // direct tasks → No Due Date
    expect(smb.subgroups[0].tasks[0]).toEqual({
      docTitle: 'task', header: '2026-11-23', text: 'do X', status: 'Pending', path: ['smthk-amdb', '2026-11-23'],
    })
  })

  it('keeps non-ISO subgroup labels in appearance order after dated ones', () => {
    const doc = (paths) => ({ tasks: [{ document_title: 'task', groups: paths.map(([h, ...rest], i) => ({
      header: h, path: [h, ...rest], tasks: [{ text: `t${i}`, status: 'Pending' }] })) }] })
    const groups = groupPending(doc([
      ['smthk-amdb', '2026-11-25'],
      ['smthk-amdb', 'reminder'],
      ['smthk-amdb', '2026-11-23'],
    ]))
    expect(groups[0].subgroups.map((s) => s.label)).toEqual(['2026-11-23', '2026-11-25', 'reminder'])
  })

  it('merges duplicate name sections across the doc, preserving subgroup appearance', () => {
    const doc = {
      tasks: [
        { document_title: 'task', groups: [
          { header: 'a', path: ['a', 'later'], tasks: [{ text: 't1', status: 'Pending' }] },
          { header: 'b', path: ['b'], tasks: [{ text: 't2', status: 'Pending' }] },
          { header: 'a', path: ['a', 'soon'], tasks: [{ text: 't3', status: 'Pending' }] },
        ] },
        { document_title: 'task2', groups: [
          { header: 'a', path: ['a', 'later'], tasks: [{ text: 't4', status: 'Pending' }] },
        ] },
      ],
    }
    const groups = groupPending(doc)
    expect(groups.map((g) => g.name)).toEqual(['a', 'b'])
    expect(groups[0].subgroups.map((s) => s.label)).toEqual(['later', 'soon']) // non-ISO keeps appearance
    expect(groups[0].subgroups[0].tasks.map((t) => t.text)).toEqual(['t1', 't4']) // merged across docs
  })

  it('returns [] for an empty response', () => {
    expect(groupPending({ tasks: [] })).toEqual([])
  })
})

describe('splitCompleted', () => {
  it('returns completed refs flat with path, in document order', () => {
    expect(splitCompleted(nestedFixture)).toEqual([
      { docTitle: 'task', header: '2026-11-23', text: 'do Y', status: 'Completed', path: ['smthk-amdb', '2026-11-23'] },
    ])
  })
  it('returns [] for an empty response', () => {
    expect(splitCompleted({ tasks: [] })).toEqual([])
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test` (from `frontend/`)
Expected: FAIL — `groupPending`/`splitCompleted` not exported (import error fails the whole suite file).

- [ ] **Step 3: Implement in `grouping.js`**

Same single-pass structure as `splitTasks`, but bucket by `path[0]` into a `Map` (insertion order = appearance order), and within each name into a `Map` keyed by sub-group label. Sorting rule for a name's sub-groups:

```js
const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/
// ISO-dated labels ascending, then others in appearance order, then No Due Date last
const dated = subgroups.filter((s) => ISO_DATE.test(s.label)).sort((a, b) => (a.label < b.label ? -1 : 1))
const rest = subgroups.filter((s) => !ISO_DATE.test(s.label) && s.label !== NO_DUE_DATE)
const none = subgroups.filter((s) => s.label === NO_DUE_DATE)
return [...dated, ...rest, ...none]
```

Skip Completed tasks in `groupPending`; skip non-Completed in `splitCompleted`. `TaskRef` carries `path`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test` (from `frontend/`)
Expected: PASS — new describes plus the untouched `splitTasks` describe.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/grouping.js frontend/src/lib/grouping.test.js
git commit -m "feat: groupPending hierarchy and splitCompleted"
```

---

### Task 4: PendingColumn UI, overflow, wiring, cleanup

**Files:**
- Create: `frontend/src/components/PendingColumn.jsx`
- Modify: `frontend/src/components/TaskCard.jsx:10,19`
- Modify: `frontend/src/components/KanbanColumn.jsx:25`
- Modify: `frontend/src/App.jsx:4,6,42-44,77-78`
- Modify: `frontend/src/lib/grouping.js` (delete `splitTasks`)
- Modify: `frontend/src/lib/grouping.test.js` (delete the `splitTasks` describe + `twoDocFixture`)
- Test: `frontend/src/components/board.test.jsx`

**Interfaces:**
- Consumes: `groupPending`/`splitCompleted` (Task 3), `NameGroup`/`SubGroup`/`TaskRef` shapes, `TaskCard`.
- Produces:
  - `PendingColumn({ title, accent, groups, loading })` — renders `Pending (N)` with N = total tasks across all groups; each name group is a collapsible section (`Button` labeled `` `${name} (${count})` `` with chevron, `aria-expanded`); sub-group labels as small muted text; tasks via `<TaskCard taskRef={…} showContext={false} />`; collapsed sections render nothing (unmount, not hide); loading shows two `Skeleton`s; content area carries `max-h-[70vh] overflow-y-auto`.
  - `TaskCard({ taskRef, showContext = true })` — context line `${docTitle} › ${path.join(' › ')}` only when `showContext`; default keeps `KanbanColumn` output identical for flat docs.
  - `KanbanColumn` — unchanged props/behavior; `CardContent` gains `max-h-[70vh] overflow-y-auto`.
  - `App` — pending column renders `<PendingColumn … groups={board.pending} />`, completed stays `<KanbanColumn … limit={10} />`; `splitTasks` deleted.

- [ ] **Step 1: Write the failing tests**

Update existing fixtures in `board.test.jsx`: every task ref gains `path: ['Home']` (the `pending`, `completed`, markdown/strikethrough/sanitize inline fixtures, and the `doneTasks` helper). Expected strings like `Tasks › Home` are unchanged — the context now comes from `path.join(' › ')`. Add one nested-path assertion to the context test:

```js
it('renders completed context with the full heading path', () => {
  const tasks = [{ docTitle: 'task', header: '2026-11-23', path: ['smthk-amdb', '2026-11-23'], text: 'Ship it', status: 'Completed' }]
  render(<KanbanColumn title="Completed" accent="green" tasks={tasks} loading={false} />)
  expect(screen.getByText('task › smthk-amdb › 2026-11-23')).toBeInTheDocument()
})
```

Add an overflow assertion to the flat column (extend or add):

```js
it('scrolls the flat column content internally', () => {
  const { container } = render(<KanbanColumn title="Completed" accent="green" tasks={doneTasks(12)} loading={false} limit={10} />)
  expect(container.querySelector('[data-slot="card-content"]')).toHaveClass('max-h-[70vh]', 'overflow-y-auto')
})
```

New `PendingColumn` describe (import it at the top):

```js
const groups = [
  { name: 'smthk-amdb', subgroups: [
    { label: '2026-11-23', tasks: [
      { docTitle: 'task', header: '2026-11-23', path: ['smthk-amdb', '2026-11-23'], text: 'do X', status: 'Pending' },
    ] },
    { label: 'No Due Date', tasks: [
      { docTitle: 'task', header: 'smthk-amdb', path: ['smthk-amdb'], text: 'do W', status: 'Pending' },
    ] },
  ] },
]

it('renders name groups with counts, subgroup labels, and no context lines', () => {
  render(<PendingColumn title="Pending" accent="amber" groups={groups} loading={false} />)
  expect(screen.getByText('Pending (2)')).toBeInTheDocument()
  expect(screen.getByText('smthk-amdb (2)')).toBeInTheDocument()
  expect(screen.getByText('2026-11-23')).toBeInTheDocument()
  expect(screen.getByText('No Due Date')).toBeInTheDocument()
  expect(screen.getByText('do X')).toBeInTheDocument()
  expect(screen.getByText('do W')).toBeInTheDocument()
  expect(screen.queryByText(/›/)).not.toBeInTheDocument() // pending cards carry no context line
})

it('collapses a group on click and restores it on the next click', () => {
  render(<PendingColumn title="Pending" accent="amber" groups={groups} loading={false} />)
  const toggle = screen.getByRole('button', { name: 'smthk-amdb (2)' })
  expect(toggle).toHaveAttribute('aria-expanded', 'true')
  fireEvent.click(toggle)
  expect(toggle).toHaveAttribute('aria-expanded', 'false')
  expect(screen.queryByText('do X')).not.toBeInTheDocument() // unmounted, not hidden
  fireEvent.click(toggle)
  expect(screen.getByText('do X')).toBeInTheDocument()
})

it('renders an empty pending column with zero count', () => {
  render(<PendingColumn title="Pending" accent="amber" groups={[]} loading={false} />)
  expect(screen.getByText('Pending (0)')).toBeInTheDocument()
})

it('scrolls the pending column content internally', () => {
  const { container } = render(<PendingColumn title="Pending" accent="amber" groups={groups} loading={false} />)
  expect(container.querySelector('[data-slot="card-content"]')).toHaveClass('max-h-[70vh]', 'overflow-y-auto')
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test` (from `frontend/`)
Expected: FAIL — `PendingColumn` import fails; existing context tests fail until fixtures carry `path`.

- [ ] **Step 3: Implement**

`TaskCard.jsx` — add the prop and swap the context line:

```jsx
export default function TaskCard({ taskRef, showContext = true }) {
  ...
  {showContext && <p className="text-xs text-muted-foreground">{`${taskRef.docTitle} › ${taskRef.path.join(' › ')}`}</p>}
```

`KanbanColumn.jsx` — only the CardContent class changes:

```jsx
<CardContent className="flex max-h-[70vh] flex-col gap-2 overflow-y-auto">
```

`PendingColumn.jsx` — exact copy (visual decisions fixed here; icons: `ChevronDown`/`ChevronRight` from `lucide-react` — if unavailable in the installed version, use `'▾'`/`'▸'` text glyphs instead):

```jsx
import { useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { ChevronDown, ChevronRight } from 'lucide-react'
import TaskCard from './TaskCard'

const ACCENT_DOT = { amber: 'bg-amber-400', green: 'bg-green-500' }

function GroupSection({ name, subgroups }) {
  const [open, setOpen] = useState(true)
  const count = subgroups.reduce((n, s) => n + s.tasks.length, 0)
  return (
    <div className="flex flex-col gap-2">
      <Button
        variant="ghost"
        size="sm"
        className="justify-start font-medium"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        {open ? <ChevronDown className="size-4" aria-hidden="true" /> : <ChevronRight className="size-4" aria-hidden="true" />}
        {`${name} (${count})`}
      </Button>
      {open && subgroups.map((s) => (
        <div key={s.label} className="flex flex-col gap-1.5 pl-1">
          <p className="text-xs font-medium text-muted-foreground">{s.label}</p>
          {s.tasks.map((t, i) => <TaskCard key={i} taskRef={t} showContext={false} />)}
        </div>
      ))}
    </div>
  )
}

export default function PendingColumn({ title, accent, groups, loading }) {
  const total = groups.reduce((n, g) => n + g.subgroups.reduce((m, s) => m + s.tasks.length, 0), 0)
  return (
    <Card size="sm" className="bg-muted/40">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <span className={`size-2 rounded-full ${ACCENT_DOT[accent] ?? 'bg-muted-foreground'}`} aria-hidden="true" />
          {`${title} (${total})`}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex max-h-[70vh] flex-col gap-3 overflow-y-auto">
        {loading ? (
          <>
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
          </>
        ) : (
          groups.map((g) => <GroupSection key={g.name} name={g.name} subgroups={g.subgroups} />)
        )}
      </CardContent>
    </Card>
  )
}
```

`App.jsx` — swap grouping imports and wiring:

```jsx
import PendingColumn from '@/components/PendingColumn'
import { groupPending, splitCompleted } from '@/lib/grouping'
// ...
const board = state.status === 'ok'
  ? { pending: groupPending(state.tasks), completed: splitCompleted(state.tasks), loaded: true }
  : { pending: [], completed: [], loaded: false }
// ...
<PendingColumn title="Pending" accent="amber" groups={board.pending} loading={!board.loaded} />
<KanbanColumn title="Completed" accent="green" tasks={board.completed} loading={!board.loaded} limit={10} />
```

Delete `splitTasks` from `grouping.js` and the `splitTasks` describe + `twoDocFixture` from `grouping.test.js`.

- [ ] **Step 4: Run the full suite to verify everything passes**

Run: `npm test` (from `frontend/`) and `cargo test` (from `backend/`)
Expected: PASS on both — whole branch green.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/PendingColumn.jsx frontend/src/components/TaskCard.jsx frontend/src/components/KanbanColumn.jsx frontend/src/components/board.test.jsx frontend/src/App.jsx frontend/src/lib/grouping.js frontend/src/lib/grouping.test.js
git commit -m "feat: pending column grouped by name and date with scrollable columns"
```
