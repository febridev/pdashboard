import { useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { ChevronDown, ChevronRight } from 'lucide-react'
import TaskCard from './TaskCard'

const ACCENT_DOT = { amber: 'bg-amber-400', green: 'bg-green-500' }

function GroupSection({ name, subgroups }) {
  const [open, setOpen] = useState(true)
  const count = subgroups.reduce((n, s) => n + s.tasks.length, 0)
  return (
    <div className="flex flex-col gap-2">
      <Button
        variant="ghost"
        size="sm"
        className="justify-start font-medium"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        {open ? <ChevronDown className="size-4" aria-hidden="true" /> : <ChevronRight className="size-4" aria-hidden="true" />}
        {`${name} (${count})`}
      </Button>
      {open && subgroups.map((s) => (
        <div key={s.label} className="flex flex-col gap-1.5 pl-1">
          {s.label && <p className="text-xs font-medium text-muted-foreground">{s.label}</p>}
          {s.tasks.map((t, i) => <TaskCard key={i} taskRef={t} />)}
        </div>
      ))}
    </div>
  )
}

export default function GroupedColumn({ title, accent, groups, loading }) {
  const total = groups.reduce((n, g) => n + g.subgroups.reduce((m, s) => m + s.tasks.length, 0), 0)
  return (
    <Card size="sm" className="bg-muted/40">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <span className={`size-2 rounded-full ${ACCENT_DOT[accent] ?? 'bg-muted-foreground'}`} aria-hidden="true" />
          {`${title} (${total})`}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex max-h-[70vh] flex-col gap-3 overflow-y-auto">
        {loading ? (
          <>
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
          </>
        ) : (
          groups.map((g) => <GroupSection key={g.name} name={g.name} subgroups={g.subgroups} />)
        )}
      </CardContent>
    </Card>
  )
}
