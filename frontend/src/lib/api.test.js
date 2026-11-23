import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ApiError, fetchTasks, parseTasksResponse } from './api'

const fixture = {
  tasks: [
    { document_title: 'Tasks', groups: [
      { header: 'Home', path: ['Home'], tasks: [
        { text: 'Buy milk', status: 'Pending' },
        { text: 'Pay rent', status: 'Completed' },
      ] },
    ] },
  ],
}

describe('parseTasksResponse', () => {
  it('accepts the API fixture', () => {
    expect(parseTasksResponse(fixture)).toEqual(fixture)
  })
  it('rejects missing tasks', () => {
    expect(() => parseTasksResponse({})).toThrow(ApiError)
  })
  it('rejects a doc without document_title', () => {
    expect(() => parseTasksResponse({ tasks: [{ groups: [] }] })).toThrow(ApiError)
  })
  it('rejects a group without path', () => {
    expect(() => parseTasksResponse({ tasks: [{ document_title: 'D', groups: [{ header: 'H', tasks: [] }] }] })).toThrow(ApiError)
  })
  it('rejects a non-string path element', () => {
    const bad = { tasks: [{ document_title: 'D', groups: [{ header: 'H', path: ['ok', 7], tasks: [] }] }] }
    expect(() => parseTasksResponse(bad)).toThrow(ApiError)
  })
  it('rejects an unknown status', () => {
    expect(() => parseTasksResponse({ tasks: [{ document_title: 'D', groups: [{ header: 'H', tasks: [{ text: 'T', status: 'done' }] }] }] })).toThrow(ApiError)
  })
})

describe('fetchTasks', () => {
  beforeEach(() => vi.unstubAllGlobals())
  it('returns parsed tasks on 200', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve(fixture) }))
    await expect(fetchTasks()).resolves.toEqual(fixture)
  })
  it('maps 401 to unauthorized', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 401 }))
    await expect(fetchTasks()).rejects.toMatchObject({ kind: 'unauthorized' })
  })
  it('maps 500 to other with HTTP detail', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500 }))
    await expect(fetchTasks()).rejects.toMatchObject({ kind: 'other', message: 'HTTP 500' })
  })
  it('maps network failure to other with err.message', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('Failed to fetch')))
    await expect(fetchTasks()).rejects.toMatchObject({ kind: 'other', message: 'Failed to fetch' })
  })
  it('maps an OK response with unparseable body to other with Invalid JSON response', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: () => Promise.reject(new SyntaxError('Unexpected token < in JSON')) }))
    await expect(fetchTasks()).rejects.toMatchObject({ kind: 'other', message: 'Invalid JSON response' })
  })
})
