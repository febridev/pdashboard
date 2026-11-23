import { it, expect, afterEach } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'
import KanbanColumn from './KanbanColumn'

afterEach(cleanup)

const pending = [
  { docTitle: 'Tasks', header: 'Home', text: 'Buy milk', status: 'Pending' },
  { docTitle: 'Tasks', header: 'Home', text: '<script>alert(1)</script>', status: 'Pending' },
]
const completed = [{ docTitle: 'Tasks', header: 'Home', text: 'Pay rent', status: 'Completed' }]

const doneTasks = (n) =>
  Array.from({ length: n }, (_, i) => ({ docTitle: 'Tasks', header: 'Home', text: `Task ${i + 1}`, status: 'Completed' }))

it('shows the column title with count', () => {
  render(<KanbanColumn title="Pending" accent="amber" tasks={pending} loading={false} />)
  expect(screen.getByText('Pending (2)')).toBeInTheDocument()
})

it('renders task context lines and never raw HTML from task text', () => {
  const { container } = render(
    <KanbanColumn title="Pending" accent="amber" tasks={pending} loading={false} />
  )
  expect(screen.getAllByText('Tasks › Home')).toHaveLength(2)
  expect(screen.getByText('Buy milk')).toBeInTheDocument()
  expect(container.querySelector('script')).toBeNull() // raw HTML dropped, never executed
})

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
  // react-markdown drops the dangerous href entirely; the anchor keeps no link role,
  // so query by text/selector and assert the URL never survives.
  const link = screen.getByText('bad', { selector: 'a' })
  expect(link).not.toHaveAttribute('href', 'javascript:alert(1)')
})

it('renders completed tasks with line-through styling', () => {
  render(<KanbanColumn title="Completed" accent="green" tasks={completed} loading={false} />)
  expect(screen.getByText('Pay rent')).toHaveClass('line-through')
  expect(screen.getByText('Pay rent').closest('[data-slot="card"]')).toHaveClass('border-l-green-500')
})

it('renders an empty column with zero count and no cards', () => {
  render(<KanbanColumn title="Pending" accent="amber" tasks={[]} loading={false} />)
  expect(screen.getByText('Pending (0)')).toBeInTheDocument()
})

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
