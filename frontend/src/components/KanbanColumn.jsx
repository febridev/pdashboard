import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import TaskCard from './TaskCard'

const ACCENT_DOT = {
  amber: 'bg-amber-400',
  green: 'bg-green-500',
}

export default function KanbanColumn({ title, accent, tasks, loading }) {
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
          tasks.map((taskRef, i) => <TaskCard key={i} taskRef={taskRef} />)
        )}
      </CardContent>
    </Card>
  )
}
