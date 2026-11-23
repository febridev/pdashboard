import { it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'
import { FakeEventSource, resetFakeEventSources } from '@/test/fakeEventSource'
import App from './App'

afterEach(cleanup)
beforeEach(() => vi.unstubAllGlobals())
// Must be registered after the unstub hook above so the stub survives it.
beforeEach(() => { vi.stubGlobal('EventSource', FakeEventSource); resetFakeEventSources() })

const task3Fixture = {
  tasks: [
    { document_title: 'Tasks', groups: [
      { header: 'Home', path: ['Home'], tasks: [
        { text: 'Buy milk', status: 'Pending' },
        { text: 'Pay rent', status: 'Completed' },
      ] },
    ] },
  ],
}

const dupFixture = {
  tasks: [
    { document_title: 'Tasks', groups: [
      { header: 'Home', path: ['Home'], tasks: [
        { text: 'Buy milk', status: 'Pending' },
        { text: 'Buy milk', status: 'Pending' },
      ] },
    ] },
  ],
}

const onePending = {
  tasks: [
    { document_title: 'Tasks', groups: [
      { header: 'Home', path: ['Home'], tasks: [
        { text: 'Buy milk', status: 'Pending' },
      ] },
    ] },
  ],
}

const twoPending = {
  tasks: [
    { document_title: 'Tasks', groups: [
      { header: 'Home', path: ['Home'], tasks: [
        { text: 'Buy milk', status: 'Pending' },
        { text: 'Pay rent', status: 'Pending' },
      ] },
    ] },
  ],
}

it('shows the Signed in bar and loading columns before the fetch resolves', async () => {
  vi.stubGlobal('fetch', vi.fn(() => new Promise(() => {}))) // never resolves
  render(<App />)
  expect(screen.getByText('Signed in')).toBeInTheDocument()
  expect(document.querySelectorAll('[data-slot="skeleton"], .animate-pulse').length).toBeGreaterThan(0)
})

it('shows the sign-in card on unauthorized', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 401 }))
  render(<App />)
  await screen.findByText('Please sign in to view your tasks')
  expect(screen.getByRole('link', { name: 'Sign in with Google' })).toHaveAttribute('href', '/auth/google')
})

it('shows the error card with the message on other failures (never sign-in)', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 502 }))
  render(<App />)
  await screen.findByText('Failed to load tasks')
  expect(screen.getByText('HTTP 502')).toBeInTheDocument()
  expect(screen.queryByText('Please sign in to view your tasks')).not.toBeInTheDocument()
})

it('renders the board from a 200 with tasks', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(task3Fixture) }))
  render(<App />)
  expect(await screen.findByText('Pending (1)')).toBeInTheDocument()
  expect(screen.getByText('Completed (1)')).toBeInTheDocument()
  expect(screen.getByRole('link', { name: 'Sign out' })).toHaveAttribute('href', '/auth/logout')
})

it('renders duplicate identical tasks without key collisions', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(dupFixture) }))
  render(<App />)
  expect(await screen.findByText('Pending (2)')).toBeInTheDocument()
  expect(screen.getAllByText('Buy milk')).toHaveLength(2)
})

it('opens no stream when the fetch is unauthorized', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 401 }))
  render(<App />)
  await screen.findByText('Please sign in to view your tasks')
  expect(FakeEventSource.instances).toHaveLength(0)
})

it('updates the board when the stream pushes new tasks', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(onePending) }))
  render(<App />)
  expect(await screen.findByText('Pending (1)')).toBeInTheDocument()
  FakeEventSource.instances.at(-1).dispatch('tasks', JSON.stringify(twoPending))
  expect(await screen.findByText('Pending (2)')).toBeInTheDocument()
})

it('renders an empty board when tasks is empty', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({ tasks: [] }) }))
  render(<App />)
  expect(await screen.findByText('Pending (0)')).toBeInTheDocument()
  expect(screen.queryByText('Failed to load tasks')).not.toBeInTheDocument()
})

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
