import { afterEach, describe, expect, it, vi } from 'vitest'
import { createDiaryAutosave } from './diaryAutosave'

afterEach(() => vi.useRealTimers())

describe('diary autosave queue', () => {
  it('debounces edits and saves only the latest snapshot', async () => {
    vi.useFakeTimers()
    const save = vi.fn(async (_snapshot: string) => undefined)
    const autosave = createDiaryAutosave(save, 250)
    autosave.schedule('older')
    autosave.schedule('latest')
    await vi.advanceTimersByTimeAsync(250)
    expect(save).toHaveBeenCalledTimes(1)
    expect(save).toHaveBeenCalledWith('latest')
    expect(autosave.status).toBe('saved')
    autosave.dispose()
  })

  it('serializes a newer snapshot behind an in-flight save', async () => {
    const first = deferred<void>()
    const save = vi.fn((snapshot: string) => snapshot === 'first' ? first.promise : Promise.resolve())
    const autosave = createDiaryAutosave(save, 250)
    autosave.schedule('first')
    const flushing = autosave.flush()
    await Promise.resolve()
    autosave.schedule('second')
    first.resolve()
    await flushing
    expect(save.mock.calls.map(([snapshot]) => snapshot)).toEqual(['first', 'second'])
    expect(autosave.status).toBe('saved')
    autosave.dispose()
  })

  it('keeps a failed snapshot retryable through flush', async () => {
    const save = vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce(undefined)
    const autosave = createDiaryAutosave(save, 250)
    autosave.schedule('retry me')
    await expect(autosave.flush()).rejects.toThrow('offline')
    expect(autosave.status).toBe('error')
    await autosave.flush()
    expect(autosave.status).toBe('saved')
    expect(save).toHaveBeenCalledTimes(2)
    autosave.dispose()
  })
})

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => { resolve = done })
  return { promise, resolve }
}
