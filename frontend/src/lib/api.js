export class ApiError extends Error {
  constructor(kind, message) {
    super(message)
    this.name = 'ApiError'
    this.kind = kind
  }
}

function fail(path, expected) {
  throw new ApiError('other', `${path}: expected ${expected}`)
}

function checkObject(value, path) {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    fail(path, 'an object')
  }
  return value
}

function checkArray(value, path) {
  if (!Array.isArray(value)) fail(path, 'an array')
  return value
}

function checkString(value, path) {
  if (typeof value !== 'string') fail(path, 'a string')
  return value
}

function checkStatus(value, path) {
  if (value !== 'Pending' && value !== 'Completed') {
    fail(path, "'Pending' or 'Completed'")
  }
  return value
}

export function parseTasksResponse(raw) {
  checkObject(raw, 'response')
  return {
    tasks: checkArray(raw.tasks, 'tasks').map((doc, i) => {
      const docPath = `tasks[${i}]`
      checkObject(doc, docPath)
      return {
        document_title: checkString(doc.document_title, `${docPath}.document_title`),
        groups: checkArray(doc.groups, `${docPath}.groups`).map((group, j) => {
          const groupPath = `${docPath}.groups[${j}]`
          checkObject(group, groupPath)
          return {
            header: checkString(group.header, `${groupPath}.header`),
            path: checkArray(group.path, `${groupPath}.path`).map((p, l) => checkString(p, `${groupPath}.path[${l}]`)),
            tasks: checkArray(group.tasks, `${groupPath}.tasks`).map((task, k) => {
              const taskPath = `${groupPath}.tasks[${k}]`
              checkObject(task, taskPath)
              return {
                text: checkString(task.text, `${taskPath}.text`),
                status: checkStatus(task.status, `${taskPath}.status`),
              }
            }),
          }
        }),
      }
    }),
  }
}

export async function fetchTasks() {
  let res
  try {
    res = await fetch('/api/tasks')
  } catch (err) {
    throw new ApiError('other', err.message)
  }
  if (res.status === 401) throw new ApiError('unauthorized', 'Not authenticated')
  if (!res.ok) throw new ApiError('other', `HTTP ${res.status}`)
  let raw
  try {
    raw = await res.json()
  } catch {
    throw new ApiError('other', 'Invalid JSON response')
  }
  return parseTasksResponse(raw)
}
