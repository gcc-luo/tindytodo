import { describe, expect, it } from 'vitest'
import { switchWorkspace, type WorkspaceState } from './diaryWorkspace'

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
})
