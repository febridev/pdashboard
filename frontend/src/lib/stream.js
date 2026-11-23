import { parseTasksResponse } from './api'

// Subscribe to the backend's SSE snapshot stream. Each `tasks` event's
// data is a TasksResponse JSON payload, validated with the same parser as
// the initial fetch. Malformed events are ignored so the board keeps its
// last good snapshot; the browser's EventSource handles reconnects, and a
// fatal close just leaves the board static until the page is reloaded.
export function subscribeTaskStream(onTasks) {
  const source = new EventSource('/api/tasks/stream')
  source.addEventListener('tasks', (event) => {
    try {
      onTasks(parseTasksResponse(JSON.parse(event.data)))
    } catch {
      // Malformed or invalid payload: skip it, keep the last good board.
    }
  })
  return () => source.close()
}
