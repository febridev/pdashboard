import { describe, it, expect, afterEach } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'
import KanbanColumn from './KanbanColumn'

afterEach(cleanup)

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
  expect(screen.getAllByText('Tasks › Home')).toHaveLength(2)
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
