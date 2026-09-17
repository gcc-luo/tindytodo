import type { TaskView, TodoList, TodoTask } from '../types'

export function localDateKey(date = new Date()): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function filterTasks(
  tasks: TodoTask[],
  view: TaskView,
  today: string,
  query = '',
  lists: TodoList[] = [],
): TodoTask[] {
  const normalizedQuery = query.trim().toLocaleLowerCase()
  const viewTasks = normalizedQuery
    ? tasks.filter((task) => {
        const listName = lists.find((list) => list.id === task.list_id)?.name ?? ''
        return [task.title, task.note ?? '', listName]
          .some((value) => value.toLocaleLowerCase().includes(normalizedQuery))
      })
    : tasks.filter((task) => {
        if (view === 'today') return task.status === 0 && !!task.due_date && task.due_date <= today
        if (view === 'inbox') return task.status === 0 && task.due_date === null
        if (view === 'planned') return task.status === 0 && !!task.due_date && task.due_date > today
        if (view === 'completed') return task.status === 1
        return task.status === 0 && task.list_id === Number(view.slice(5))
      })

  return [...viewTasks].sort((a, b) => a.sort_order - b.sort_order || a.id - b.id)
}

export function countTasks(tasks: TodoTask[], view: TaskView, today: string): number {
  if (view === 'today') return tasks.filter((task) => task.status === 0 && !!task.due_date && task.due_date <= today).length
  if (view === 'inbox') return tasks.filter((task) => task.status === 0 && task.due_date === null).length
  if (view === 'planned') return tasks.filter((task) => task.status === 0 && !!task.due_date && task.due_date > today).length
  if (view === 'completed') return tasks.filter((task) => task.status === 1).length
  return tasks.filter((task) => task.status === 0 && task.list_id === Number(view.slice(5))).length
}
