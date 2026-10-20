import type { TaskView } from '../types'

export interface WorkspaceState {
  activeWorkspace: 'tasks' | 'diary'
  taskView: TaskView
  selectedDate: string
}

export function switchWorkspace(state: WorkspaceState, activeWorkspace: WorkspaceState['activeWorkspace']): WorkspaceState {
  return { ...state, activeWorkspace }
}
