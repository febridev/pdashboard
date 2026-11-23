import { describe, it, expect } from 'vitest'
import { splitTasks, groupPending, splitCompleted } from './grouping'

// reuse the Task 3 fixture shape, plus a second doc to pin cross-doc order:
// doc 'Tasks' group 'Home' → Buy milk (Pending), Pay rent (Completed)
// doc 'Work' group 'Infra' → Ship release (Pending)
const twoDocFixture = {
  tasks: [
    {
      document_title: 'Tasks',
      groups: [
        {
          header: 'Home',
          tasks: [
            { text: 'Buy milk', status: 'Pending' },
            { text: 'Pay rent', status: 'Completed' },
          ],
        },
      ],
    },
    {
      document_title: 'Work',
      groups: [
        {
          header: 'Infra',
          tasks: [{ text: 'Ship release', status: 'Pending' }],
        },
      ],
    },
  ],
}

describe('splitTasks', () => {
  it('splits by status preserving document/group/task order', () => {
    const { pending, completed } = splitTasks(twoDocFixture)
    expect(pending.map((t) => t.text)).toEqual(['Buy milk', 'Ship release'])
    expect(completed.map((t) => t.text)).toEqual(['Pay rent'])
    expect(pending[1]).toEqual({ docTitle: 'Work', header: 'Infra', text: 'Ship release', status: 'Pending' })
  })

  it('returns two empty arrays when tasks is empty', () => {
    expect(splitTasks({ tasks: [] })).toEqual({ pending: [], completed: [] })
  })
})

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
