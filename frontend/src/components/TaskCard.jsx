import { Card, CardContent } from '@/components/ui/card'

export default function TaskCard({ taskRef }) {
  const completed = taskRef.status === 'Completed'

  return (
    <Card
      size="sm"
      className={completed ? 'border-l-4 border-l-green-500' : 'border-l-4 border-l-amber-400'}
    >
      <CardContent className="flex flex-col gap-1">
        <p className="text-xs text-muted-foreground">{`${taskRef.docTitle} › ${taskRef.header}`}</p>
        <p className={completed ? 'text-sm text-muted-foreground line-through' : 'text-sm'}>
          {taskRef.text}
        </p>
      </CardContent>
    </Card>
  )
}
