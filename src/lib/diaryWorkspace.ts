import type { TaskView } from '../types'

export interface WorkspaceState {
  activeWorkspace: 'tasks' | 'diary'
  taskView: TaskView
  selectedDate: string
}

export function switchWorkspace(state: WorkspaceState, activeWorkspace: WorkspaceState['activeWorkspace']): WorkspaceState {
  return { ...state, activeWorkspace }
}

export async function flushBeforeAppExit(
  activeWorkspace: WorkspaceState['activeWorkspace'],
  flushDiary: () => Promise<boolean>,
  exit: () => Promise<void>,
): Promise<boolean> {
  if (activeWorkspace === 'diary' && !await flushDiary()) return false
  await exit()
  return true
}
