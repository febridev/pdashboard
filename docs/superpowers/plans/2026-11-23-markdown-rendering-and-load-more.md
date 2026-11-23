# Markdown Rendering + Completed "Load More" Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render the inline markdown inside Outline task text on the Kanban cards, and cap the Completed column at 10 visible tasks with a "Load more" button.

**Architecture:** Pure frontend change. `TaskCard.jsx` renders `taskRef.text` through `react-markdown` (XSS-safe by default — raw HTML is dropped, never executed). `KanbanColumn.jsx` gains an optional `limit` prop that slices the task list and appends a reveal button; `App.jsx` passes `limit={10}` only to the Completed column. Backend untouched — the Rust parser already passes task text through verbatim, which is what we want.

**Tech Stack:** React 19 + Vite (plain JSX, no TypeScript), shadcn/ui components, Tailwind v4, Vitest + @testing-library/react (jsdom), `react-markdown` v10 (new dependency).

**Spec:** `docs/superpowers/specs/2026-11-23-markdown-rendering-and-load-more-design.md`

## Global Constraints

- Plain JavaScript/JSX only — no TypeScript syntax anywhere.
- No backend changes; no changes to `src/lib/grouping.js` or `src/lib/api.js` (their tests must stay green).
- XSS safety is non-negotiable: raw HTML in task text must never become DOM elements; `javascript:` link URLs must never survive as an href.
- All commands run from `frontend/`. Full suite: `npm test`. Single file: `npx vitest run src/components/board.test.jsx`.
- Commit only files the task names.

## Review Focus

Input classes the spec implies but the happy path doesn't cover — each pinned to a test below:

1. **Raw HTML in task text** (`<script>alert(1)</script>`) must never appear as a DOM element. → Task 1, Step 1 `never raw HTML` test asserts `container.querySelector('script')` is null.
2. **Dangerous link protocol** (`[bad](javascript:alert(1))`) must never survive as an href. → Task 1, Step 1 protocol test asserts `not.toHaveAttribute('href', 'javascript:alert(1)')`.
3. **External links must open in a new tab.** → Task 1, Step 1 markdown test asserts `target="_blank"` and `rel="noreferrer"`.
4. **Exactly at the limit (10 tasks)** is the off-by-one boundary: all visible, no button. → Task 2, Step 1 boundary test.
5. **Duplicate identical task texts** must keep rendering once per task through both changes. → Existing `renders duplicate identical tasks without key collisions` test in `src/App.test.jsx` must stay green after every task (it guards against text-node and key collapse).

---

### Task 1: Markdown rendering in TaskCard

**Files:**
- Modify: `frontend/package.json` (add `react-markdown` dependency)
- Modify: `frontend/src/components/TaskCard.jsx`
- Modify: `frontend/src/components/board.test.jsx`

**Interfaces:**
- Consumes: `taskRef` prop, shape unchanged (`{ docTitle, header, text, status }`).
- Produces: `TaskCard` renders `taskRef.text` as markdown; export name and signature unchanged, so `KanbanColumn` needs no edits in this task.

- [ ] **Step 1: Write the failing tests**

In `src/components/board.test.jsx`, replace the test `renders task context lines and literal task text` with:

```jsx
it('renders task context lines and never raw HTML from task text', () => {
  const { container } = render(
    <KanbanColumn title="Pending" accent="amber" tasks={pending} loading={false} />
  )
  expect(screen.getAllByText('Tasks › Home')).toHaveLength(2)
  expect(screen.getByText('Buy milk')).toBeInTheDocument()
  expect(container.querySelector('script')).toBeNull() // raw HTML dropped, never executed
})
```

…and add two new tests (keep the existing `pending` fixture, which contains the `<script>` task):

```jsx
it('renders inline markdown in task text', () => {
  const tasks = [{
    docTitle: 'Tasks', header: 'Home',
    text: 'Fix **login** flow — see [docs](https://example.com/guide) and `v2` notes',
    status: 'Pending',
  }]
  render(<KanbanColumn title="Pending" accent="amber" tasks={tasks} loading={false} />)
  expect(screen.getByText('login', { selector: 'strong' })).toBeInTheDocument()
  const link = screen.getByRole('link', { name: 'docs' })
  expect(link).toHaveAttribute('href', 'https://example.com/guide')
  expect(link).toHaveAttribute('target', '_blank')
  expect(link).toHaveAttribute('rel', 'noreferrer')
  expect(screen.getByText('v2', { selector: 'code' })).toBeInTheDocument()
})

it('sanitizes dangerous link protocols in task text', () => {
  const tasks = [{ docTitle: 'Tasks', header: 'Home', text: 'Never visit [bad](javascript:alert(1))', status: 'Pending' }]
  render(<KanbanColumn title="Pending" accent="amber" tasks={tasks} loading={false} />)
  expect(screen.getByRole('link', { name: 'bad' })).not.toHaveAttribute('href', 'javascript:alert(1)')
})
```

Leave `renders completed tasks with line-through styling` and the empty-column test untouched — they must keep passing against the new implementation (the markdown paragraph carries the completed/pending classes, so `getByText('Pay rent').toHaveClass('line-through')` still works).

- [ ] **Step 2: Run to verify the new tests fail**

Run: `npx vitest run src/components/board.test.jsx`
Expected: the two new markdown tests FAIL (no `strong`, no link role — text renders literally); the replaced raw-HTML test FAILS on `container.querySelector('script')`… note the current implementation renders the literal string, so this test may PASS pre-change — the decisive red signals are the two markdown tests.

- [ ] **Step 3: Install react-markdown and implement TaskCard**

Run: `npm install react-markdown` (v10; React 19 compatible — it lands in `package.json` on its own).

Rewrite `frontend/src/components/TaskCard.jsx` to render `taskRef.text` through `<ReactMarkdown>` with this components map (styling decisions fixed here; Tailwind v4 preflight already zeroes `p` margins and keeps `strong`/`em` defaults):

```jsx
const INLINE_MD = {
  a: (props) => <a {...props} target="_blank" rel="noreferrer" className="underline underline-offset-2" />,
  code: (props) => <code {...props} className="rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]" />,
}
```

The completed/pending classes move from the current static `<p>` onto a `p` component override, so `line-through` covers the whole rendered line:

```jsx
p: (props) => (
  <p {...props} className={completed ? 'text-sm text-muted-foreground line-through' : 'text-sm'} />
)
```

If React warns about an unknown `node` prop reaching a DOM element, destructure it out before spreading (`({ node, ...props })`).

- [ ] **Step 4: Run the full suite to verify green**

Run: `npm test`
Expected: all pass — board tests (including `renders completed tasks with line-through styling` unchanged) AND App tests unchanged, including `renders duplicate identical tasks without key collisions`.

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json src/components/TaskCard.jsx src/components/board.test.jsx
git commit -m "feat: render inline markdown in task text via react-markdown"
```

### Task 2: Completed column "Load more"

**Files:**
- Modify: `frontend/src/components/KanbanColumn.jsx`
- Modify: `frontend/src/App.jsx:78` (Completed column)
- Modify: `frontend/src/components/board.test.jsx`
- Modify: `frontend/src/App.test.jsx`

**Interfaces:**
- Consumes: `TaskCard` from Task 1 (unchanged interface).
- Produces: `KanbanColumn({ title, accent, tasks, loading, limit })` — `limit` optional; when set and exceeded, shows `tasks.slice(0, limit + extra)` plus a ghost Button labeled ``Load more (${hidden})`` that adds `limit` more per click; header count stays `tasks.length`. `App.jsx` passes `limit={10}` to the Completed column only.

- [ ] **Step 1: Write the failing tests**

In `src/components/board.test.jsx`, import `fireEvent` from `@testing-library/react`, add a fixture helper, then four tests:

```jsx
const doneTasks = (n) =>
  Array.from({ length: n }, (_, i) => ({ docTitle: 'Tasks', header: 'Home', text: `Task ${i + 1}`, status: 'Completed' }))

it('caps a limited column at the limit and reveals the rest on Load more', () => {
  render(<KanbanColumn title="Completed" accent="green" tasks={doneTasks(12)} loading={false} limit={10} />)
  expect(screen.getByText('Completed (12)')).toBeInTheDocument() // header keeps the true total
  expect(screen.getByText('Task 10')).toBeInTheDocument()
  expect(screen.queryByText('Task 11')).not.toBeInTheDocument()
  fireEvent.click(screen.getByRole('button', { name: 'Load more (2)' }))
  expect(screen.getByText('Task 12')).toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Load more' })).not.toBeInTheDocument()
})

it('reveals limit-sized pages and updates the hidden count', () => {
  render(<KanbanColumn title="Completed" accent="green" tasks={doneTasks(23)} loading={false} limit={10} />)
  fireEvent.click(screen.getByRole('button', { name: 'Load more (13)' }))
  expect(screen.getByText('Task 20')).toBeInTheDocument()
  expect(screen.queryByText('Task 21')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Load more (3)' })).toBeInTheDocument()
})

it('shows no Load more when the column fits within the limit', () => {
  render(<KanbanColumn title="Completed" accent="green" tasks={doneTasks(10)} loading={false} limit={10} />)
  expect(screen.getByText('Task 10')).toBeInTheDocument()
  expect(screen.queryByRole('button', { name: /Load more/ })).not.toBeInTheDocument()
})

it('renders all tasks when no limit is given', () => {
  render(<KanbanColumn title="Pending" accent="amber" tasks={doneTasks(12)} loading={false} />)
  expect(screen.getByText('Task 12')).toBeInTheDocument()
  expect(screen.queryByRole('button', { name: /Load more/ })).not.toBeInTheDocument()
})
```

In `src/App.test.jsx`, add the wiring test (proves `limit={10}` reaches the Completed column):

```jsx
it('caps the Completed column at 10 with Load more', async () => {
  const manyDone = {
    tasks: [
      { document_title: 'Tasks', groups: [
        { header: 'Home', tasks: Array.from({ length: 12 }, (_, i) => ({ text: `Done ${i + 1}`, status: 'Completed' })) },
      ] },
    ],
  }
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(manyDone) }))
  render(<App />)
  expect(await screen.findByText('Completed (12)')).toBeInTheDocument()
  expect(screen.getByText('Done 10')).toBeInTheDocument()
  expect(screen.queryByText('Done 11')).not.toBeInTheDocument()
})
```

- [ ] **Step 2: Run to verify the new tests fail**

Run: `npm test`
Expected: all five new tests FAIL (no limit prop exists — 12th task visible, no button). Existing tests still pass.

- [ ] **Step 3: Implement the limit prop in KanbanColumn and wire it in App**

`KanbanColumn.jsx`: import `useState` and `Button` from `@/components/ui/button`; compute the slice (the whole algorithm — state grows by `limit` per click, `hidden` recomputed from the slice):

```jsx
const [extra, setExtra] = useState(0)
const visibleTasks = limit ? tasks.slice(0, limit + extra) : tasks
const hidden = limit ? tasks.length - visibleTasks.length : 0
```

Render `visibleTasks.map(...)` (keep the existing `key={i}` pattern), and after the list, while any remain hidden:

```jsx
{hidden > 0 && (
  <Button variant="ghost" size="sm" onClick={() => setExtra((e) => e + limit)}>
    {`Load more (${hidden})`}
  </Button>
)}
```

Header count stays `` `${title} (${tasks.length})` ``. Skeleton loading branch unchanged.

`App.jsx` line 78: add `limit={10}` to the Completed `<KanbanColumn>`. Pending unchanged.

- [ ] **Step 4: Run the full suite to verify green**

Run: `npm test`
Expected: all pass, including every pre-existing test.

- [ ] **Step 5: Commit**

```bash
git add src/components/KanbanColumn.jsx src/App.jsx src/components/board.test.jsx src/App.test.jsx
git commit -m "feat: cap Completed column at 10 with Load more"
```
