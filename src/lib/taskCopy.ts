import type { TodoList, TodoTask } from '../types'

export function formatTaskForCopy(task: TodoTask): string {
  const lines = [`- [${task.status === 1 ? 'x' : ' '}] ${task.title}${task.priority === 1 ? '（高优先级）' : ''}`]
  if (task.due_date || task.due_time) {
    lines.push(`  截止：${[task.due_date, task.due_time].filter(Boolean).join(' ')}`)
  }
  if (task.note?.trim()) {
    lines.push(`  备注：${task.note.trim().replace(/\n/g, '\n  ')}`)
  }
  return lines.join('\n')
}

export function formatListForCopy(list: TodoList, tasks: TodoTask[]): string {
  return [`清单：${list.name}（${tasks.length} 个任务）`, '', ...tasks.map(formatTaskForCopy)].join('\n')
}
