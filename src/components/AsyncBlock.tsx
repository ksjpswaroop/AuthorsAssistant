import type { ReactNode } from 'react'
import type { Async } from '../lib/useAsync'
import { Callout, EmptyState } from './ui'
import { color } from '../lib/tokens'

/**
 * Renders the three states every real data view has. Being explicit about
 * "empty" versus "failed" matters: an author seeing nothing needs to know
 * whether that is an answer or a problem.
 */
export function AsyncBlock<T>({
  state,
  empty,
  isEmpty,
  children,
}: {
  state: Async<T>
  empty: { title: string; body?: string }
  isEmpty?: (data: T) => boolean
  children: (data: T) => ReactNode
}) {
  if (state.loading) {
    return <div style={{ padding: 24, fontSize: 13, color: color.ink4 }}>Working…</div>
  }
  if (state.error) {
    return (
      <Callout tone="warn" icon="alert" style={{ margin: 20 }}>
        {state.error}
      </Callout>
    )
  }
  if (!state.data || (isEmpty?.(state.data) ?? false)) {
    return <EmptyState icon="doc" title={empty.title} body={empty.body} />
  }
  return <>{children(state.data)}</>
}
