export type DiarySaveStatus = 'idle' | 'saving' | 'saved' | 'error'

export interface DiaryAutosave<T> {
  readonly status: DiarySaveStatus
  schedule(snapshot: T): void
  flush(): Promise<void>
  dispose(): void
}

export function createDiaryAutosave<T>(
  save: (snapshot: T) => Promise<unknown>,
  delay = 500,
  onStatus?: (status: DiarySaveStatus, error?: unknown) => void,
): DiaryAutosave<T> {
  let status: DiarySaveStatus = 'idle'
  let timer: ReturnType<typeof setTimeout> | undefined
  let snapshot: T
  let revision = 0
  let dirty = false
  let inFlight: Promise<void> | undefined
  let disposed = false

  const setStatus = (next: DiarySaveStatus, error?: unknown) => {
    status = next
    onStatus?.(next, error)
  }

  const clearTimer = () => {
    if (timer !== undefined) clearTimeout(timer)
    timer = undefined
  }

  const flush = async (): Promise<void> => {
    clearTimer()
    if (disposed) return
    if (inFlight) await inFlight.catch(() => undefined)
    if (!dirty) return

    const current = snapshot!
    const currentRevision = revision
    dirty = false
    setStatus('saving')
    const attempt = Promise.resolve().then(() => save(current)).then(() => undefined)
    inFlight = attempt
    try {
      await attempt
      if (!dirty && currentRevision === revision) setStatus('saved')
    } catch (error) {
      if (currentRevision === revision) {
        dirty = true
        setStatus('error', error)
        throw error
      }
    } finally {
      inFlight = undefined
    }

    if (dirty) await flush()
  }

  return {
    get status() { return status },
    schedule(nextSnapshot: T) {
      if (disposed) return
      snapshot = nextSnapshot
      revision += 1
      dirty = true
      clearTimer()
      timer = setTimeout(() => { void flush().catch(() => undefined) }, delay)
    },
    flush,
    dispose() {
      disposed = true
      clearTimer()
    },
  }
}
