import { Card, CardContent } from '@/components/ui/card'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

const INLINE_MD = {
  a: (props) => <a {...props} target="_blank" rel="noreferrer" className="underline underline-offset-2" />,
  code: (props) => <code {...props} className="rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]" />,
}

export default function TaskCard({ taskRef, showContext = true }) {
  const completed = taskRef.status === 'Completed'

  return (
    <Card
      size="sm"
      className={completed ? 'border-l-4 border-l-green-500' : 'border-l-4 border-l-amber-400'}
    >
      <CardContent className="flex flex-col gap-1">
        {showContext && <p className="text-xs text-muted-foreground">{`${taskRef.docTitle} › ${taskRef.path.join(' › ')}`}</p>}
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            ...INLINE_MD,
            p: (props) => (
              <p {...props} className={completed ? 'text-sm text-muted-foreground line-through' : 'text-sm'} />
            ),
          }}
        >
          {taskRef.text}
        </ReactMarkdown>
      </CardContent>
    </Card>
  )
}
