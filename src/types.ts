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

export interface DiaryEntry {
  id: number
  entry_date: string
  title: string
  content_html: string
  created_at: string
  updated_at: string
  deleted_at: string | null
  tag_ids: number[]
}

export interface DiaryTag {
  id: number
  name: string
  created_at: string
}

export interface DiaryAsset {
  id: string
  entry_id: number | null
  relative_path: string
  mime_type: string
  pending_delete: boolean
}

export interface DiaryData {
  entries: DiaryEntry[]
  tags: DiaryTag[]
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
