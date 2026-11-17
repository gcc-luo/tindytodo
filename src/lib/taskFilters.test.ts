import { describe, expect, it } from 'vitest'
import { countTasks, filterTasks } from './taskFilters'
import type { TodoTask } from '../types'

const tasks: TodoTask[] = [
  { id: 1, title: '今天的任务', note: null, list_id: 1, priority: 0, due_date: '2026-09-17', due_time: null, status: 0, sort_order: 2, created_at: '', updated_at: '', completed_at: null },
  { id: 2, title: '逾期任务', note: '需要尽快做', list_id: null, priority: 1, due_date: '2026-09-16', due_time: null, status: 0, sort_order: 1, created_at: '', updated_at: '', completed_at: null },
  { id: 3, title: '收集箱任务', note: null, list_id: null, priority: 0, due_date: null, due_time: null, status: 0, sort_order: 3, created_at: '', updated_at: '', completed_at: null },
  { id: 4, title: '之后再做', note: null, list_id: null, priority: 0, due_date: '2026-09-20', due_time: null, status: 0, sort_order: 4, created_at: '', updated_at: '', completed_at: null },
  { id: 5, title: '已完成任务', note: null, list_id: null, priority: 0, due_date: null, due_time: null, status: 1, sort_order: 5, created_at: '', updated_at: '', completed_at: '' },
]

describe('task view filters', () => {
  it('shows overdue and due-today open tasks in Today, ordered by manual order', () => {
    expect(filterTasks(tasks, 'today', '2026-09-17').map((task) => task.id)).toEqual([2, 1])
  })

  it('keeps undated tasks in Inbox and future tasks in Planned', () => {
    expect(filterTasks(tasks, 'inbox', '2026-09-17').map((task) => task.id)).toEqual([3])
    expect(filterTasks(tasks, 'planned', '2026-09-17').map((task) => task.id)).toEqual([4])
  })

  it('searches titles, notes, and list names across completed tasks too', () => {
    expect(filterTasks(tasks, 'inbox', '2026-09-17', '尽快').map((task) => task.id)).toEqual([2])
    expect(filterTasks(tasks, 'today', '2026-09-17', '已完成').map((task) => task.id)).toEqual([5])
  })

  it('filters completed tasks by list or no list while preserving global search', () => {
    const extra: TodoTask[] = [
      { ...tasks[4], id: 6, title: '工作完成', list_id: 1 },
      { ...tasks[4], id: 7, title: '生活完成', list_id: 2 },
    ]
    const all = [...tasks, ...extra]
    expect(filterTasks(all, 'completed', '2026-09-17', '', [], 1).map((task) => task.id)).toEqual([6])
    expect(filterTasks(all, 'completed', '2026-09-17', '', [], null).map((task) => task.id)).toEqual([5])
    expect(filterTasks(all, 'completed', '2026-09-17').map((task) => task.id)).toEqual([5, 6, 7])
    expect(filterTasks(all, 'completed', '2026-09-17', '今天', [], 1).map((task) => task.id)).toEqual([1])
  })

  it('counts overdue open tasks in Today', () => {
    expect(countTasks(tasks, 'today', '2026-09-17')).toBe(2)
  })
})
