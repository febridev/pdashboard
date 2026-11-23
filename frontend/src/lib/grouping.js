/**
 * @typedef {{ docTitle: string, header: string, text: string, status: 'Pending' | 'Completed', path: string[] }} TaskRef
 */

const NO_DUE_DATE = 'No Due Date'
const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/

/**
 * Groups pending tasks into a name → sub-group hierarchy.
 * Name = path[0]; sub-group label = path.slice(1).join(' › '), or 'No Due Date' for direct tasks.
 * Name groups keyed by first appearance (merged across docs by name string); within a name:
 * ISO-dated labels ascending first, then non-ISO labels in appearance order, then No Due Date last.
 *
 * @param {{ tasks: Array<{ document_title: string, groups: Array<{ header: string, path: string[], tasks: Array<{ text: string, status: 'Pending' | 'Completed' }> }> }> }} tasksResponse
 * @returns {Array<{ name: string, subgroups: Array<{ label: string, tasks: TaskRef[] }> }>}
 */
export function groupPending(tasksResponse) {
  const byName = new Map()

  for (const doc of tasksResponse.tasks) {
    for (const group of doc.groups) {
      for (const task of group.tasks) {
        if (task.status === 'Completed') continue

        const name = group.path[0]
        const label = group.path.length > 1 ? group.path.slice(1).join(' › ') : NO_DUE_DATE

        if (!byName.has(name)) {
          byName.set(name, new Map())
        }
        const subgroups = byName.get(name)
        if (!subgroups.has(label)) {
          subgroups.set(label, [])
        }
        subgroups.get(label).push({
          docTitle: doc.document_title,
          header: group.header,
          text: task.text,
          status: task.status,
          path: group.path,
        })
      }
    }
  }

  const groups = []
  for (const [name, subgroupsByName] of byName) {
    const subgroups = []
    for (const [label, tasks] of subgroupsByName) {
      subgroups.push({ label, tasks })
    }

    // ISO-dated labels ascending, then others in appearance order, then No Due Date last
    const dated = subgroups.filter((s) => ISO_DATE.test(s.label)).sort((a, b) => (a.label < b.label ? -1 : 1))
    const rest = subgroups.filter((s) => !ISO_DATE.test(s.label) && s.label !== NO_DUE_DATE)
    const none = subgroups.filter((s) => s.label === NO_DUE_DATE)
    groups.push({ name, subgroups: [...dated, ...rest, ...none] })
  }

  return groups
}

/**
 * Groups completed tasks into a name → single unlabeled subgroup hierarchy.
 * Name = path[0]; sub-group label = null (one subgroup per name, holding all its completed tasks).
 * Name groups keyed by first appearance (merged across docs by name string); tasks in document
 * order within each subgroup. No date logic — completed tasks are not date-sorted.
 *
 * @param {{ tasks: Array<{ document_title: string, groups: Array<{ header: string, path: string[], tasks: Array<{ text: string, status: 'Pending' | 'Completed' }> }> }> }} tasksResponse
 * @returns {Array<{ name: string, subgroups: Array<{ label: null, tasks: TaskRef[] }> }>}
 */
export function groupCompleted(tasksResponse) {
  const byName = new Map()

  for (const doc of tasksResponse.tasks) {
    for (const group of doc.groups) {
      for (const task of group.tasks) {
        if (task.status !== 'Completed') continue

        const name = group.path[0]

        if (!byName.has(name)) {
          byName.set(name, [])
        }
        byName.get(name).push({
          docTitle: doc.document_title,
          header: group.header,
          text: task.text,
          status: task.status,
          path: group.path,
        })
      }
    }
  }

  return Array.from(byName, ([name, tasks]) => ({ name, subgroups: [{ label: null, tasks }] }))
}
