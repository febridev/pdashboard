import { describe, it, expect } from 'vitest'
import { splitTasks } from './grouping'

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
