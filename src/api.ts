import { emit, listen } from '@tauri-apps/api/event'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { AppData, DiaryData, DiaryEntry, DiaryTag, TaskDraft, TaskChanges, TodoList, TodoTask, WorkSettings } from './types'
import * as diaryBrowserStore from './lib/diaryBrowserStore'
import type { DiaryAssetBlob } from './lib/diaryBrowserStore'
import { referencedDiaryAssetIds } from './lib/diaryEditor'

const STORAGE_KEY = 'tinytodo-local-v1'
const WORK_SETTINGS_KEY = 'tinytodo-work-settings-v1'
const nativeApp = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
export const isNativeApp = nativeApp

export interface ShortcutInfo {
  label: string
  registered: boolean
}

export const defaultWorkSettings = (): WorkSettings => ({
  lunch: { enabled: false, time: '12:00', repeat: 'daily', date: null, completed: false },
  offWork: { enabled: false, time: '18:00', repeat: 'daily', date: null, completed: false },
})

function readWorkSettings(): WorkSettings {
  const saved = localStorage.getItem(WORK_SETTINGS_KEY)
  if (!saved) return defaultWorkSettings()
  try {
    const parsed = JSON.parse(saved) as Partial<WorkSettings>
    const defaults = defaultWorkSettings()
    return {
      lunch: { ...defaults.lunch, ...parsed.lunch },
      offWork: { ...defaults.offWork, ...parsed.offWork },
    }
  } catch {
    return defaultWorkSettings()
  }
}

export async function getWorkSettings(): Promise<WorkSettings> {
  return nativeApp ? invoke<WorkSettings>('get_work_settings') : readWorkSettings()
}

export async function saveWorkSettings(settings: WorkSettings): Promise<void> {
  if (nativeApp) await invoke('save_work_settings', { settings })
  else localStorage.setItem(WORK_SETTINGS_KEY, JSON.stringify(settings))
}

function readLocalData(): AppData {
  const saved = localStorage.getItem(STORAGE_KEY)
  if (saved) return JSON.parse(saved) as AppData
  const now = new Date().toISOString()
  const initial: AppData = {
    tasks: [],
    lists: [
      { id: 1, name: '工作', sort_order: 0, created_at: now },
      { id: 2, name: '学习', sort_order: 1, created_at: now },
      { id: 3, name: '生活', sort_order: 2, created_at: now },
    ],
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(initial))
  return initial
}

function saveLocalData(data: AppData): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
}

function nextId(items: Array<{ id: number }>): number {
  return Math.max(0, ...items.map((item) => item.id)) + 1
}

function localTimestamp(): string {
  return new Date().toISOString()
}

export async function getAppData(): Promise<AppData> {
  return nativeApp ? invoke<AppData>('get_app_data') : readLocalData()
}

export async function getDiaryData(): Promise<DiaryData> {
  return nativeApp
    ? invoke<DiaryData>('get_diary_data')
    : diaryBrowserStore.getDiaryData()
}

export async function createDiaryEntry(entryDate: string): Promise<DiaryEntry> {
  return nativeApp
    ? invoke<DiaryEntry>('create_diary_entry', { entryDate })
    : diaryBrowserStore.createDiaryEntry(entryDate)
}

export async function saveDiaryEntry(id: number, title: string, contentHtml: string, tagIds: number[]): Promise<DiaryEntry> {
  const entry = nativeApp
    ? invoke<DiaryEntry>('save_diary_entry', { id, title, contentHtml, tagIds })
    : diaryBrowserStore.saveDiaryEntry(id, title, contentHtml, tagIds)
  const saved = await entry
  await pruneDiaryAssets(id, referencedDiaryAssetIds(contentHtml))
  return saved
}

export async function putDiaryAsset(asset: DiaryAssetBlob): Promise<void> {
  if (nativeApp) {
    await invoke('put_diary_asset', {
      id: asset.id,
      entryId: asset.entryId,
      mimeType: asset.mimeType,
      bytes: Array.from(asset.bytes),
    })
  } else {
    await diaryBrowserStore.putDiaryAsset(asset)
  }
}

export async function importDiaryImage(entryId: number): Promise<{ id: string; src: string; mimeType: string } | null> {
  if (!nativeApp) return null
  const path = await open({
    multiple: false,
    filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
  })
  if (typeof path !== 'string') return null
  const id = crypto.randomUUID()
  const asset = await invoke<{ id: string; mime_type: string }>('import_diary_asset', { id, entryId, path })
  const src = await resolveDiaryAsset(asset.id)
  if (!src) throw new Error('图片保存后无法读取')
  return { id: asset.id, src, mimeType: asset.mime_type }
}

export async function resolveDiaryAsset(id: string): Promise<string | null> {
  if (nativeApp) {
    const path = await invoke<string>('resolve_diary_asset', { id })
    return convertFileSrc(path)
  }
  return diaryBrowserStore.resolveDiaryAsset(id)
}

export async function deleteDiaryAsset(id: string): Promise<void> {
  if (nativeApp) await invoke('delete_diary_asset', { id })
  else await diaryBrowserStore.deleteDiaryAsset(id)
}

export async function pruneDiaryAssets(entryId: number, keepIds: string[]): Promise<void> {
  if (nativeApp) await invoke('prune_diary_assets', { entryId, keepIds })
  else await diaryBrowserStore.pruneDiaryAssets(entryId, keepIds)
}

export async function createDiaryTag(name: string): Promise<DiaryTag> {
  return nativeApp
    ? invoke<DiaryTag>('create_diary_tag', { name })
    : diaryBrowserStore.createDiaryTag(name)
}

export async function renameDiaryTag(id: number, name: string): Promise<DiaryTag> {
  return nativeApp
    ? invoke<DiaryTag>('rename_diary_tag', { id, name })
    : diaryBrowserStore.renameDiaryTag(id, name)
}

export async function deleteDiaryTag(id: number): Promise<void> {
  if (nativeApp) await invoke('delete_diary_tag', { id })
  else await diaryBrowserStore.deleteDiaryTag(id)
}

export async function trashDiaryEntry(id: number): Promise<void> {
  if (nativeApp) await invoke('trash_diary_entry', { id })
  else await diaryBrowserStore.trashDiaryEntry(id)
}

export async function restoreDiaryEntry(id: number): Promise<void> {
  if (nativeApp) await invoke('restore_diary_entry', { id })
  else await diaryBrowserStore.restoreDiaryEntry(id)
}

export async function purgeDiaryEntry(id: number): Promise<void> {
  if (nativeApp) await invoke('purge_diary_entry', { id })
  else await diaryBrowserStore.purgeDiaryEntry(id)
}

export async function getShortcutInfo(): Promise<ShortcutInfo> {
  return nativeApp
    ? invoke<ShortcutInfo>('get_shortcut_info')
    : { label: 'Ctrl + Alt + Space', registered: true }
}

export async function setShortcut(label: string): Promise<ShortcutInfo> {
  return nativeApp
    ? invoke<ShortcutInfo>('set_shortcut', { label })
    : { label, registered: true }
}

async function notifyChanged(): Promise<void> {
  if (nativeApp) await emit('tasks:changed')
  else window.dispatchEvent(new Event('tasks:changed'))
}

export async function createTask(draft: TaskDraft): Promise<TodoTask> {
  if (nativeApp) {
    const task = await invoke<TodoTask>('create_task', { draft })
    await notifyChanged()
    return task
  }
  const data = readLocalData()
  const now = localTimestamp()
  const task: TodoTask = {
    id: nextId(data.tasks), title: draft.title.trim(), note: draft.note?.trim() || null,
    list_id: draft.list_id ?? null, priority: draft.priority ?? 0,
    due_date: draft.due_date || null, due_time: draft.due_time || null,
    status: 0, sort_order: Math.max(-1, ...data.tasks.map((item) => item.sort_order)) + 1,
    created_at: now, updated_at: now, completed_at: null,
  }
  data.tasks.push(task)
  saveLocalData(data)
  await notifyChanged()
  return task
}

export async function updateTask(id: number, changes: TaskChanges, current: TodoTask): Promise<TodoTask> {
  const draft: TaskDraft = {
    title: changes.title ?? current.title,
    note: changes.note === undefined ? current.note : changes.note,
    list_id: changes.list_id === undefined ? current.list_id : changes.list_id,
    priority: changes.priority ?? current.priority,
    due_date: changes.due_date === undefined ? current.due_date : changes.due_date,
    due_time: changes.due_time === undefined ? current.due_time : changes.due_time,
  }
  if (nativeApp) {
    const task = await invoke<TodoTask>('update_task', { id, draft })
    await notifyChanged()
    return task
  }
  const data = readLocalData()
  const index = data.tasks.findIndex((task) => task.id === id)
  if (index < 0) throw new Error('找不到这个任务，它可能已经被删除')
  const task = { ...data.tasks[index], ...draft, title: draft.title.trim(), updated_at: localTimestamp() }
  data.tasks[index] = task
  saveLocalData(data)
  await notifyChanged()
  return task
}

export async function toggleTask(id: number): Promise<TodoTask> {
  if (nativeApp) {
    const task = await invoke<TodoTask>('toggle_task', { id })
    await notifyChanged()
    return task
  }
  const data = readLocalData()
  const task = data.tasks.find((item) => item.id === id)
  if (!task) throw new Error('找不到这个任务，它可能已经被删除')
  task.status = task.status === 0 ? 1 : 0
  task.completed_at = task.status === 1 ? localTimestamp() : null
  task.updated_at = localTimestamp()
  saveLocalData(data)
  await notifyChanged()
  return task
}

export async function deleteTask(id: number): Promise<void> {
  if (nativeApp) await invoke('delete_task', { id })
  else {
    const data = readLocalData()
    data.tasks = data.tasks.filter((task) => task.id !== id)
    saveLocalData(data)
  }
  await notifyChanged()
}

export async function reorderTasks(ids: number[]): Promise<void> {
  if (nativeApp) await invoke('reorder_tasks', { ids })
  else {
    const data = readLocalData()
    ids.forEach((id, index) => {
      const task = data.tasks.find((item) => item.id === id)
      if (task) task.sort_order = index
    })
    saveLocalData(data)
  }
  await notifyChanged()
}

export async function createList(name: string): Promise<TodoList> {
  if (nativeApp) {
    const list = await invoke<TodoList>('create_list', { name })
    await notifyChanged()
    return list
  }
  const data = readLocalData()
  if (data.lists.some((list) => list.name.toLocaleLowerCase() === name.trim().toLocaleLowerCase())) {
    throw new Error('已有同名清单，请换一个名称')
  }
  const list = { id: nextId(data.lists), name: name.trim(), sort_order: data.lists.length, created_at: localTimestamp() }
  data.lists.push(list)
  saveLocalData(data)
  await notifyChanged()
  return list
}

export async function renameList(id: number, name: string): Promise<TodoList> {
  if (nativeApp) {
    const list = await invoke<TodoList>('rename_list', { id, name })
    await notifyChanged()
    return list
  }
  const data = readLocalData()
  if (data.lists.some((list) => list.id !== id && list.name.toLocaleLowerCase() === name.trim().toLocaleLowerCase())) {
    throw new Error('已有同名清单，请换一个名称')
  }
  const list = data.lists.find((item) => item.id === id)
  if (!list) throw new Error('找不到这个清单')
  list.name = name.trim()
  saveLocalData(data)
  await notifyChanged()
  return list
}

export async function deleteList(id: number): Promise<void> {
  if (nativeApp) await invoke('delete_list', { id })
  else {
    const data = readLocalData()
    data.lists = data.lists.filter((list) => list.id !== id)
    data.tasks = data.tasks.map((task) => task.list_id === id ? { ...task, list_id: null } : task)
    saveLocalData(data)
  }
  await notifyChanged()
}

export async function subscribeToChanges(callback: () => void): Promise<() => void> {
  if (nativeApp) return listen('tasks:changed', callback)
  window.addEventListener('tasks:changed', callback)
  return () => window.removeEventListener('tasks:changed', callback)
}

export async function subscribeToQuickAdd(callback: () => void): Promise<() => void> {
  if (!nativeApp) return () => undefined
  return listen('quick-add:show', callback)
}

export async function hideQuickAdd(): Promise<void> {
  if (nativeApp) await invoke('hide_quick_add')
  else window.location.href = '/'
}
