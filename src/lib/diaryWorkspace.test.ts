import { describe, expect, it } from 'vitest'
import { flushBeforeAppExit, switchWorkspace, type WorkspaceState } from './diaryWorkspace'

const initial: WorkspaceState = {
  activeWorkspace: 'tasks',
  taskView: 'list:3',
  selectedDate: '2026-10-20',
}

describe('workspace switching', () => {
  it('keeps the last diary date when opening the diary workspace', () => {
    expect(switchWorkspace(initial, 'diary')).toEqual({ ...initial, activeWorkspace: 'diary' })
  })

  it('returns to the existing task view without changing task navigation state', () => {
    const diary = switchWorkspace(initial, 'diary')
    expect(switchWorkspace(diary, 'tasks')).toEqual(initial)
  })

  it('flushes the diary before exiting and keeps the app open when saving fails', async () => {
    const events: string[] = []
    const exited = await flushBeforeAppExit('diary', async () => {
      events.push('flush')
      return false
    }, async () => { events.push('exit') })

    expect(exited).toBe(false)
    expect(events).toEqual(['flush'])
  })

  it('exits directly from the task workspace', async () => {
    const events: string[] = []
    const exited = await flushBeforeAppExit('tasks', async () => {
      events.push('flush')
      return true
    }, async () => { events.push('exit') })

    expect(exited).toBe(true)
    expect(events).toEqual(['exit'])
  })
})
