import { describe, it, expect, afterEach } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'
import GroupedColumn from './GroupedColumn'

afterEach(cleanup)

const pending = [
  { docTitle: 'Tasks', header: 'Home', path: ['Home'], text: 'Buy milk', status: 'Pending' },
  { docTitle: 'Tasks', header: 'Home', path: ['Home'], text: '<script>alert(1)</script>', status: 'Pending' },
]
const completed = [{ docTitle: 'Tasks', header: 'Home', path: ['Home'], text: 'Pay rent', status: 'Completed' }]

const wrap = (name, tasks) => [{ name, subgroups: [{ label: null, tasks }] }]

it('renders task cards and never raw HTML from task text', () => {
  const { container } = render(
    <GroupedColumn title="Pending" accent="amber" groups={wrap('Home', pending)} loading={false} />
  )
  expect(screen.getByText('Buy milk')).toBeInTheDocument()
  expect(container.querySelector('script')).toBeNull() // raw HTML dropped, never executed
})

it('renders inline markdown in task text', () => {
  const tasks = [{
    docTitle: 'Tasks', header: 'Home', path: ['Home'],
    text: 'Fix **login** flow — see [docs](https://example.com/guide) and `v2` notes',
    status: 'Pending',
  }]
  render(<GroupedColumn title="Pending" accent="amber" groups={wrap('Home', tasks)} loading={false} />)
  expect(screen.getByText('login', { selector: 'strong' })).toBeInTheDocument()
  const link = screen.getByRole('link', { name: 'docs' })
  expect(link).toHaveAttribute('href', 'https://example.com/guide')
  expect(link).toHaveAttribute('target', '_blank')
  expect(link).toHaveAttribute('rel', 'noreferrer')
  expect(screen.getByText('v2', { selector: 'code' })).toBeInTheDocument()
})

it('renders GFM strikethrough in task text', () => {
  const tasks = [{ docTitle: 'Tasks', header: 'Home', path: ['Home'], text: 'Ship ~~v1~~ v2 launch', status: 'Pending' }]
  render(<GroupedColumn title="Pending" accent="amber" groups={wrap('Home', tasks)} loading={false} />)
  expect(screen.getByText('v1', { selector: 'del' })).toBeInTheDocument()
})

it('sanitizes dangerous link protocols in task text', () => {
  const tasks = [{ docTitle: 'Tasks', header: 'Home', path: ['Home'], text: 'Never visit [bad](javascript:alert(1))', status: 'Pending' }]
  render(<GroupedColumn title="Pending" accent="amber" groups={wrap('Home', tasks)} loading={false} />)
  // react-markdown drops the dangerous href entirely; the anchor keeps no link role,
  // so query by text/selector and assert the URL never survives.
  const link = screen.getByText('bad', { selector: 'a' })
  expect(link).not.toHaveAttribute('href', 'javascript:alert(1)')
})

it('renders completed tasks with line-through styling', () => {
  render(<GroupedColumn title="Completed" accent="green" groups={wrap('Home', completed)} loading={false} />)
  expect(screen.getByText('Pay rent')).toHaveClass('line-through')
  expect(screen.getByText('Pay rent').closest('[data-slot="card"]')).toHaveClass('border-l-green-500')
})

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

const completedGroups = wrap('smthk-amdb', [
  { docTitle: 'task', header: '2026-11-23', path: ['smthk-amdb', '2026-11-23'], text: 'do Y', status: 'Completed' },
])

describe('GroupedColumn', () => {
  it('renders name groups with counts, subgroup labels, and no context lines', () => {
    render(<GroupedColumn title="Pending" accent="amber" groups={groups} loading={false} />)
    expect(screen.getByText('Pending (2)')).toBeInTheDocument()
    expect(screen.getByText('smthk-amdb (2)')).toBeInTheDocument()
    expect(screen.getByText('2026-11-23')).toBeInTheDocument()
    expect(screen.getByText('No Due Date')).toBeInTheDocument()
    expect(screen.getByText('do X')).toBeInTheDocument()
    expect(screen.getByText('do W')).toBeInTheDocument()
    expect(screen.queryByText(/›/)).not.toBeInTheDocument() // pending cards carry no context line
  })

  it('collapses a group on click and restores it on the next click', () => {
    render(<GroupedColumn title="Pending" accent="amber" groups={groups} loading={false} />)
    const toggle = screen.getByRole('button', { name: 'smthk-amdb (2)' })
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    fireEvent.click(toggle)
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByText('do X')).not.toBeInTheDocument() // unmounted, not hidden
    fireEvent.click(toggle)
    expect(screen.getByText('do X')).toBeInTheDocument()
  })

  it('renders an empty pending column with zero count', () => {
    render(<GroupedColumn title="Pending" accent="amber" groups={[]} loading={false} />)
    expect(screen.getByText('Pending (0)')).toBeInTheDocument()
  })

  it('scrolls the pending column content internally', () => {
    const { container } = render(<GroupedColumn title="Pending" accent="amber" groups={groups} loading={false} />)
    expect(container.querySelector('[data-slot="card-content"]')).toHaveClass('max-h-[70vh]', 'overflow-y-auto')
  })

  it('renders completed name groups without subgroup labels', () => {
    render(<GroupedColumn title="Completed" accent="green" groups={completedGroups} loading={false} />)
    expect(screen.getByText('Completed (1)')).toBeInTheDocument()
    expect(screen.getByText('smthk-amdb (1)')).toBeInTheDocument()
    expect(screen.getByText('do Y')).toBeInTheDocument()
    expect(screen.queryByText('No Due Date')).not.toBeInTheDocument() // label:null renders no label paragraph
    expect(screen.queryByText(/›/)).not.toBeInTheDocument()
  })
})
