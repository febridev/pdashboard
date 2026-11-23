import { describe, it, expect } from 'vitest'
import { groupCompleted, groupPending, splitCompleted } from './grouping'

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
