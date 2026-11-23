/**
 * Flattens a validated TasksResponse into the two Kanban columns.
 * Order preserved: document order → group order → task order, within each column.
 *
 * @param {{ tasks: Array<{ document_title: string, groups: Array<{ header: string, tasks: Array<{ text: string, status: 'Pending' | 'Completed' }> }> }> }} tasksResponse
 * @returns {{ pending: TaskRef[], completed: TaskRef[] }}
 *
 * @typedef {{ docTitle: string, header: string, text: string, status: 'Pending' | 'Completed' }} TaskRef
 */
export function splitTasks(tasksResponse) {
  const pending = []
  const completed = []

  for (const doc of tasksResponse.tasks) {
    for (const group of doc.groups) {
      for (const task of group.tasks) {
        const ref = {
          docTitle: doc.document_title,
          header: group.header,
          text: task.text,
          status: task.status,
        }
        if (task.status === 'Completed') {
          completed.push(ref)
        } else {
          pending.push(ref)
        }
      }
    }
  }

  return { pending, completed }
}
