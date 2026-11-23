import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import KanbanColumn from '@/components/KanbanColumn'
import PendingColumn from '@/components/PendingColumn'
import { ApiError, fetchTasks } from '@/lib/api'
import { groupPending, splitCompleted } from '@/lib/grouping'

function StatusBar() {
  return (
    <div className="mb-6 flex items-center justify-between">
      <span className="text-sm text-muted-foreground">Signed in</span>
      <Button asChild variant="ghost" size="sm">
        <a href="/auth/logout">Sign out</a>
      </Button>
    </div>
  )
}

function CenteredCard({ children }) {
  return (
    <div className="flex min-h-[60vh] items-center justify-center">
      <Card className="w-full max-w-sm">{children}</Card>
    </div>
  )
}

export default function App() {
  const [state, setState] = useState({ status: 'loading', message: '', tasks: null })

  useEffect(() => {
    fetchTasks()
      .then((tasks) => setState({ status: 'ok', message: '', tasks }))
      .catch((err) => {
        if (err instanceof ApiError && err.kind === 'unauthorized') {
          setState({ status: 'unauthorized', message: '', tasks: null })
        } else {
          setState({ status: 'error', message: err.message, tasks: null })
        }
      })
  }, [])

  const board = state.status === 'ok'
    ? { pending: groupPending(state.tasks), completed: splitCompleted(state.tasks), loaded: true }
    : { pending: [], completed: [], loaded: false }

  return (
    <main className="min-h-screen bg-background p-6">
      {state.status === 'unauthorized' && (
        <CenteredCard>
          <CardHeader>
            <CardTitle>Personal Dashboard</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <p className="text-muted-foreground">Please sign in to view your tasks</p>
            <Button asChild>
              <a href="/auth/google">Sign in with Google</a>
            </Button>
          </CardContent>
        </CenteredCard>
      )}

      {state.status === 'error' && (
        <CenteredCard>
          <CardHeader>
            <CardTitle>Failed to load tasks</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="break-words text-muted-foreground">{state.message}</p>
          </CardContent>
        </CenteredCard>
      )}

      {(state.status === 'loading' || state.status === 'ok') && (
        <>
          <StatusBar />
          <div className="grid gap-6 md:grid-cols-2">
            <PendingColumn title="Pending" accent="amber" groups={board.pending} loading={!board.loaded} />
            <KanbanColumn title="Completed" accent="green" tasks={board.completed} loading={!board.loaded} limit={10} />
          </div>
        </>
      )}
    </main>
  )
}
