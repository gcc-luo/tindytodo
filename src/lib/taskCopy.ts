import type { TodoTask } from '../types'

export function formatTaskForCopy(task: TodoTask): string {
  return task.title
}

export function formatListForCopy(tasks: TodoTask[]): string {
  return tasks.map(formatTaskForCopy).join('\n')
}
