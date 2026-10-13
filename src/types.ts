export type TaskPriority = 0 | 1
export type TaskStatus = 0 | 1
export type ReminderRepeat = 'daily' | 'once'

export interface WorkReminderSettings {
  enabled: boolean
  time: string
  repeat: ReminderRepeat
  date: string | null
  completed: boolean
}

export interface WorkSettings {
  lunch: WorkReminderSettings
  offWork: WorkReminderSettings
}

export interface WorkReportEntry {
  id: string
  date: string
  content: string
  progress: number
  task_id: number | null
  created_at: string
  updated_at: string
}

export interface WorkReports {
  entries: WorkReportEntry[]
}

export interface TodoTask {
  id: number
  title: string
  note: string | null
  list_id: number | null
  priority: TaskPriority
  due_date: string | null
  due_time: string | null
  status: TaskStatus
  sort_order: number
  created_at: string
  updated_at: string
  completed_at: string | null
}

export interface TodoList {
  id: number
  name: string
  sort_order: number
  created_at: string
}

export interface AppData {
  tasks: TodoTask[]
  lists: TodoList[]
}

export type TaskView = 'today' | 'inbox' | 'planned' | 'completed' | `list:${number}`

export interface TaskDraft {
  title: string
  note?: string | null
  list_id?: number | null
  priority?: TaskPriority
  due_date?: string | null
  due_time?: string | null
}

export type TaskChanges = Partial<TaskDraft>
