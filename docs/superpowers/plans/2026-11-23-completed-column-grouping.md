# Completed Column Name-Grouping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Group the Completed column by task name (collapsible, no date sub-groups) by generalizing `PendingColumn` into a single `GroupedColumn` used by both columns, removing Load more and the now-dead flat column and context lines.

**Architecture:** `groupCompleted` emits the same unified groups shape as `groupPending` but with one unlabeled subgroup per name (`label: null`); `PendingColumn.jsx` becomes `GroupedColumn.jsx` whose label paragraph renders only when non-null, so Pending renders unchanged and Completed shows bare tasks under each name; `TaskCard` loses its context line (no consumer shows it anymore); `KanbanColumn.jsx` and `splitCompleted` are deleted as dead code.

**Tech Stack:** React 19 + Tailwind v4 + vitest + @testing-library/react (frontend only; backend untouched).

**Spec:** `docs/superpowers/specs/2026-11-23-completed-column-grouping-design.md`

## Global Constraints

- Backend untouched — `path` already flows; run backend commands from `backend/`, frontend commands from `frontend/`.
- No new dependencies; group labels are plain strings rendered as text, never markdown/HTML.
- jsdom does no layout: overflow is asserted via class names on `[data-slot="card-content"]`.
- Pending behavior is frozen: the four existing `PendingColumn` describe tests in `board.test.jsx` must keep passing against `GroupedColumn` with identical rendered output (component rename only).
- `App.test.jsx`: exactly one test changes — 'caps the Completed column at 10 with Load more' is replaced; the other six must pass unmodified.
- Suite arithmetic: frontend starts 41/41; Task 1 ends 44/44; Task 2 ends 35/35. Backend stays 3/3.
- Commit only the files each task names.

## Review Focus

1. **Completed tasks under flat headers (no date nesting) land under the header as the name** — `path: ['b']` → name `b`, same as Pending's flat-doc behavior. → Task 1, `merges duplicate names…` uses `path: ['a']` and `path: ['a','d2']` in one group.
2. **The same name across docs merges, document order preserved within** → Task 1, `merges duplicate names and keeps document order`.
3. **A `label: null` subgroup renders no label paragraph — no `No Due Date` or date leaks onto Completed** → Task 2, `renders completed name groups without subgroup labels`.
4. **Pending rendering is unchanged by the conditional-label change** — date labels and No Due Date still render → Task 2, the four kept describe tests (pinned by the frozen-behavior constraint).
5. **Dead code fully removed** — no dangling imports of `KanbanColumn`/`PendingColumn`/`splitCompleted`, no unused `showContext` prop → Task 2 verification step runs the named grep; a dangling import fails the suite loudly.

---

### Task 1: `groupCompleted` data layer

**Files:**
- Modify: `frontend/src/lib/grouping.js` (add `groupCompleted` after `groupPending`; `splitCompleted` stays until Task 2)
- Test: `frontend/src/lib/grouping.test.js` (add describe; `splitCompleted` describe stays until Task 2)

**Interfaces:**
- Consumes: validated `TasksResponse` (groups carry `path: string[]`, ≥ 1 entry).
- Produces: `groupCompleted(tasksResponse) → Array<{ name: string, subgroups: Array<{ label: null, tasks: TaskRef[] }> }>` — Completed-only; name = `path[0]`; names keyed by first appearance merged across docs; tasks in document order inside the single unlabeled subgroup; `TaskRef = { docTitle, header, text, status, path }`.

- [ ] **Step 1: Write the failing tests**

Import `{ groupCompleted, groupPending }` from `./grouping`. Add after the existing `groupPending` describe (the `nestedFixture` at the top of the file is reused):

```js
describe('groupCompleted', () => {
  it('groups completed by name with one unlabeled subgroup, document order within', () => {
    const groups = groupCompleted(nestedFixture)
    expect(groups).toEqual([
      { name: 'smthk-amdb', subgroups: [{ label: null, tasks: [
        { docTitle: 'task', header: '2026-11-23', text: 'do Y', status: 'Completed', path: ['smthk-amdb', '2026-11-23'] },
      ] }] },
    ])
  })

  it('merges duplicate names across docs and keeps document order', () => {
    const doc = {
      tasks: [
        { document_title: 't1', groups: [
          { header: 'a', path: ['a'], tasks: [{ text: 'c1', status: 'Completed' }] },
          { header: 'b', path: ['b', 'd1'], tasks: [{ text: 'c2', status: 'Completed' }, { text: 'p1', status: 'Pending' }] },
        ] },
        { document_title: 't2', groups: [
          { header: 'a', path: ['a', 'd2'], tasks: [{ text: 'c3', status: 'Completed' }] },
        ] },
      ],
    }
    const groups = groupCompleted(doc)
    expect(groups.map((g) => g.name)).toEqual(['a', 'b'])
    expect(groups[0].subgroups).toEqual([{ label: null, tasks: [
      { docTitle: 't1', header: 'a', text: 'c1', status: 'Completed', path: ['a'] },
      { docTitle: 't2', header: 'a', text: 'c3', status: 'Completed', path: ['a', 'd2'] },
    ] }])
    expect(groups[1].subgroups[0].tasks.map((t) => t.text)).toEqual(['c2']) // pending p1 excluded
  })

  it('returns [] for an empty response', () => {
    expect(groupCompleted({ tasks: [] })).toEqual([])
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run src/lib/grouping.test.js` (from `frontend/`)
Expected: FAIL — `groupCompleted` is not exported (import error fails the file).

- [ ] **Step 3: Implement `groupCompleted` in `frontend/src/lib/grouping.js`**

Same triple-loop structure as `splitCompleted`, but bucket Completed refs into a `Map` keyed by `path[0]` (insertion order = appearance), then map each entry to `{ name, subgroups: [{ label: null, tasks }] }`:

```js
return Array.from(byName, ([name, tasks]) => ({ name, subgroups: [{ label: null, tasks }] }))
```

JSDoc mirrors `groupPending`'s, stating: name = `path[0]`; single unlabeled subgroup; document order; no date logic.

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test` (from `frontend/`)
Expected: PASS — 44/44 (41 + 3 new).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/grouping.js frontend/src/lib/grouping.test.js
git commit -m "feat: groupCompleted name groups with unlabeled subgroups"
```

---

### Task 2: GroupedColumn unification, wiring, and dead-code removal

**Files:**
- Create: `frontend/src/components/GroupedColumn.jsx` (renamed from `PendingColumn.jsx`; three line changes specified below)
- Delete: `frontend/src/components/PendingColumn.jsx`, `frontend/src/components/KanbanColumn.jsx`
- Modify: `frontend/src/components/TaskCard.jsx:10,19` (drop `showContext` and the context line)
- Modify: `frontend/src/App.jsx:4-7,44,78-79` (wire both columns to `GroupedColumn` + `groupCompleted`)
- Modify: `frontend/src/lib/grouping.js` (delete `splitCompleted` + its JSDoc)
- Test: `frontend/src/components/board.test.jsx` (rewrite per steps), `frontend/src/lib/grouping.test.js` (delete the `splitCompleted` describe and its import), `frontend/src/App.test.jsx` (replace one test)

**Interfaces:**
- Consumes: `groupCompleted` (Task 1) and the existing `groupPending`; `TaskCard({ taskRef })`.
- Produces: `GroupedColumn({ title, accent, groups, loading })` where `groups: Array<{ name, subgroups: Array<{ label: string | null, tasks: TaskRef[] }> }>` — renders the label paragraph only when `label` is non-null; everything else identical to today's `PendingColumn` (collapse shell, counts, `max-h-[70vh] overflow-y-auto`, loading skeletons). App renders it for both columns; no `limit` prop exists anywhere.

- [ ] **Step 1: Rewrite the component-level tests (RED)**

In `board.test.jsx`: change the `PendingColumn` import to `import GroupedColumn from './GroupedColumn'`; inside the describe, rename `PendingColumn` → `GroupedColumn` and the describe title to `GroupedColumn`; the four tests' bodies and assertions stay identical. Add a fixture helper and one new completed test:

```js
const wrap = (name, tasks) => [{ name, subgroups: [{ label: null, tasks }] }]
const completedGroups = wrap('smthk-amdb', [
  { docTitle: 'task', header: '2026-11-23', path: ['smthk-amdb', '2026-11-23'], text: 'do Y', status: 'Completed' },
])
```

```js
it('renders completed name groups without subgroup labels', () => {
  render(<GroupedColumn title="Completed" accent="green" groups={completedGroups} loading={false} />)
  expect(screen.getByText('Completed (1)')).toBeInTheDocument()
  expect(screen.getByText('smthk-amdb (1)')).toBeInTheDocument()
  expect(screen.getByText('do Y')).toBeInTheDocument()
  expect(screen.queryByText('No Due Date')).not.toBeInTheDocument() // label:null renders no label paragraph
  expect(screen.queryByText(/›/)).not.toBeInTheDocument()
})
```

Then rewrite the five flat-column tests that survive (delete `KanbanColumn` import, the top-level flat fixtures, and the eight tests that die: title-count, completed-context-path, empty-flat, the four Load-more/no-limit tests, scrolls-flat):
- 'renders task cards and never raw HTML from task text' — `groups={wrap('Home', pending)}` (the existing `pending` fixture with the `<script>` task); keep `expect(screen.getByText('Buy milk')).toBeInTheDocument()` and `expect(container.querySelector('script')).toBeNull()`; delete the context-line assertion.
- 'renders inline markdown in task text', 'renders GFM strikethrough in task text', 'sanitizes dangerous link protocols in task text' — same fixtures/tasks as today, rendered via `groups={wrap('Home', tasks)}` on `title="Pending" accent="amber"`; every assertion unchanged.
- 'renders completed tasks with line-through styling' — `groups={wrap('Home', completed)}` on `title="Completed" accent="green"`; both assertions (`line-through` class, `border-l-green-500` on the card) unchanged.

In `grouping.test.js`: import becomes `{ groupCompleted, groupPending }`; delete the `splitCompleted` describe (2 tests).

In `App.test.jsx`: replace only 'caps the Completed column at 10 with Load more' with:

```js
it('renders completed tasks grouped by name without Load more', async () => {
  const manyDone = {
    tasks: [
      { document_title: 'Tasks', groups: [
        { header: 'Home', path: ['Home'], tasks: Array.from({ length: 12 }, (_, i) => ({ text: `Done ${i + 1}`, status: 'Completed' })) },
      ] },
    ],
  }
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(manyDone) }))
  render(<App />)
  expect(await screen.findByText('Completed (12)')).toBeInTheDocument()
  expect(screen.getByText('Done 12')).toBeInTheDocument() // all visible; collapse + scroll handle length
  expect(screen.queryByRole('button', { name: /Load more/ })).not.toBeInTheDocument()
})
```

The other six App tests are untouched.

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test` (from `frontend/`)
Expected: FAIL — `GroupedColumn` import fails the board suite; App's replaced test fails on the Load-more button (still rendered); grouping import fails on the deleted `splitCompleted`.

- [ ] **Step 3: Implement**

`GroupedColumn.jsx` = today's `PendingColumn.jsx` with exactly three changes: (1) component renamed `PendingColumn` → `GroupedColumn` (the private `GroupSection` keeps its name); (2) the subgroup label line becomes conditional — `{s.label && <p className="text-xs font-medium text-muted-foreground">{s.label}</p>}`; (3) the task line becomes `<TaskCard key={i} taskRef={t} />`. Delete `PendingColumn.jsx` and `KanbanColumn.jsx`.

`TaskCard.jsx`: signature becomes `TaskCard({ taskRef })`; delete the context `<p>` line (`showContext` and the `${docTitle} › ${path.join(' › ')}` line both go).

`App.jsx`: imports become `import GroupedColumn from '@/components/GroupedColumn'` and `import { groupCompleted, groupPending } from '@/lib/grouping'`; `board.completed = groupCompleted(state.tasks)`; both columns render `<GroupedColumn … groups={board.pending|completed} loading={!board.loaded} />` (Completed uses `accent="green"`; the `limit` prop is gone).

`grouping.js`: delete `splitCompleted` and its JSDoc.

- [ ] **Step 4: Verify no dangling references, then run the full suites**

Run from repo root: `grep -rn "KanbanColumn\|PendingColumn\|splitCompleted\|showContext" frontend/src`
Expected: zero matches in `frontend/src` (the plan itself and docs/ are out of scope).

Run: `npm test` (from `frontend/`) and `cargo test` (from `backend/`)
Expected: frontend 35/35 (api 11, grouping 7, board 10, App 7); backend 3/3.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/GroupedColumn.jsx frontend/src/components/TaskCard.jsx frontend/src/App.jsx frontend/src/lib/grouping.js frontend/src/components/board.test.jsx frontend/src/lib/grouping.test.js frontend/src/App.test.jsx
git rm frontend/src/components/PendingColumn.jsx frontend/src/components/KanbanColumn.jsx
git commit -m "feat: completed column grouped by name via shared GroupedColumn"
```
