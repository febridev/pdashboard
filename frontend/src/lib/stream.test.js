import { it, expect, vi, beforeEach, afterEach } from 'vitest'
import { FakeEventSource, resetFakeEventSources } from '@/test/fakeEventSource'
import { subscribeTaskStream } from './stream'

beforeEach(() => { vi.stubGlobal('EventSource', FakeEventSource); resetFakeEventSources() })
afterEach(() => vi.unstubAllGlobals())

const payload = { tasks: [{ document_title: 'T', groups: [{ header: 'H', path: ['H'], tasks: [{ text: 'a', status: 'Pending' }] }] }] }

it('subscribes to /api/tasks/stream and delivers parsed snapshots', () => {
  const seen = []
  subscribeTaskStream((tasks) => seen.push(tasks))
  const es = FakeEventSource.instances.at(-1)
  expect(es.url).toBe('/api/tasks/stream')
  es.dispatch('tasks', JSON.stringify(payload))
  expect(seen).toHaveLength(1)
  expect(seen[0].tasks[0].groups[0].tasks[0].text).toBe('a')
})

it('ignores malformed JSON and invalid payloads without throwing', () => {
  const seen = []
  subscribeTaskStream((tasks) => seen.push(tasks))
  const es = FakeEventSource.instances.at(-1)
  expect(() => es.dispatch('tasks', 'not json')).not.toThrow()
  expect(() => es.dispatch('tasks', JSON.stringify({ tasks: [{ document_title: 5 }] }))).not.toThrow()
  expect(seen).toHaveLength(0)
})

it('closing the unsubscribe function closes the connection', () => {
  const unsubscribe = subscribeTaskStream(() => {})
  const es = FakeEventSource.instances.at(-1)
  unsubscribe()
  expect(es.closed).toBe(true)
})
