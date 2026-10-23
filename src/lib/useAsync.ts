import { useCallback, useEffect, useRef, useState } from 'react'

export interface Async<T> {
  data: T | null
  error: string | null
  loading: boolean
  reload: () => void
}

/**
 * Runs a project-scoped query and keeps the result. Errors are captured rather
 * than thrown: a screen that cannot load its data should say so in place, not
 * take the window down.
 */
export function useAsync<T>(run: () => Promise<T>, deps: unknown[], enabled = true): Async<T> {
  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(enabled)
  const [tick, setTick] = useState(0)
  // A stale response from a previous document must not overwrite a newer one.
  const seq = useRef(0)

  const reload = useCallback(() => setTick((t) => t + 1), [])

  useEffect(() => {
    if (!enabled) {
      setLoading(false)
      return
    }
    const mine = ++seq.current
    setLoading(true)
    setError(null)
    run()
      .then((value) => {
        if (seq.current === mine) {
          setData(value)
          setLoading(false)
        }
      })
      .catch((e: unknown) => {
        if (seq.current === mine) {
          setError(e instanceof Error ? e.message : String(e))
          setLoading(false)
        }
      })
    // `run` is intentionally not a dependency: callers pass an inline closure.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, enabled, tick])

  return { data, error, loading, reload }
}
