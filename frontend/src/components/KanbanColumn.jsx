import { useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import TaskCard from './TaskCard'

const ACCENT_DOT = {
  amber: 'bg-amber-400',
  green: 'bg-green-500',
}

export default function KanbanColumn({ title, accent, tasks, loading, limit }) {
  const [extra, setExtra] = useState(0)
  const visibleTasks = limit ? tasks.slice(0, limit + extra) : tasks
  const hidden = limit ? tasks.length - visibleTasks.length : 0

  return (
    <Card size="sm" className="bg-muted/40">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <span className={`size-2 rounded-full ${ACCENT_DOT[accent] ?? 'bg-muted-foreground'}`} aria-hidden="true" />
          {`${title} (${tasks.length})`}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        {loading ? (
          <>
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
          </>
        ) : (
          <>
            {visibleTasks.map((taskRef, i) => <TaskCard key={i} taskRef={taskRef} />)}
            {hidden > 0 && (
              <Button variant="ghost" size="sm" onClick={() => setExtra((e) => e + limit)}>
                {`Load more (${hidden})`}
              </Button>
            )}
          </>
        )}
      </CardContent>
    </Card>
  )
}
