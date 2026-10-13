<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { check, type DownloadEvent } from '@tauri-apps/plugin-updater'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  Archive, CalendarDays, Check, CheckCheck, ChevronRight, Clock3,
  FileText, Inbox, ListPlus, MoreHorizontal, Plus, Search, Settings2, Sparkles, Sun, X,
} from 'lucide-vue-next'
import TaskDetails from './components/TaskDetails.vue'
import TaskRow from './components/TaskRow.vue'
import ReportsPage from './components/ReportsPage.vue'
import QuickAddWindow from './components/QuickAddWindow.vue'
import {
  createList, createTask, deleteList, deleteTask, getAppData, getShortcutInfo, getWorkReports, reorderTasks, saveWorkReports, setShortcut,
  renameList, subscribeToChanges, toggleTask, updateTask, defaultWorkReports, defaultWorkSettings, getWorkSettings, saveWorkSettings,
} from './api'
import { countTasks, filterTasks, localDateKey } from './lib/taskFilters'
import { crossedOccurrence, formatCountdown, nextReminder, type ReminderKind } from './lib/workReminders'
import type { AppData, TaskDraft, TaskView, TodoList, TaskChanges, TodoTask, WorkReportEntry, WorkReports, WorkSettings } from './types'

type AppView = TaskView | 'reports'

const isQuickAdd = new URLSearchParams(window.location.search).has('quickAdd')
const nativeApp = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const appData = ref<AppData>({ tasks: [], lists: [] })
const loading = ref(true)
const activeView = ref<AppView>('today')
const selectedId = ref<number | null>(null)
const searchOpen = ref(false)
const searchQuery = ref('')
const toast = ref('')
const shortcutLabel = ref('Ctrl + Alt + Space')
const shortcutRegistered = ref(true)
const settingsOpen = ref(false)
const updateStatus = ref('')
const checkingForUpdates = ref(false)
const updateProgress = ref<number | null>(null)
const workSettings = ref<WorkSettings>(defaultWorkSettings())
const settingsDraft = ref<WorkSettings>(defaultWorkSettings())
const workReports = ref<WorkReports>(defaultWorkReports())
const reportSyncTask = ref<TodoTask | null>(null)
const reportSyncProgress = ref(0)
const reportSyncOpen = ref(false)
const reminderNow = ref(new Date())
const celebration = ref<ReminderKind | null>(null)
const addTitle = ref('')
const addOptionsOpen = ref(false)
const addDate = ref(localDateKey())
const addTime = ref('18:00')
const addListId = ref('')
const addPriority = ref(false)
const addInput = ref<HTMLInputElement | null>(null)
const newListOpen = ref(false)
const newListName = ref('')
const listMenuId = ref<number | null>(null)
const draggingId = ref<number | null>(null)
const dragTargetId = ref<number | null>(null)
let unlisten: (() => void) | undefined
let toastTimer: ReturnType<typeof setTimeout> | undefined
let dayTimer: ReturnType<typeof setInterval> | undefined
let reminderTimer: ReturnType<typeof setInterval> | undefined
let previousReminderTime: Date | null = null
let reminderTickBusy = false
const triggeredReminders = new Set<string>()

const today = ref(localDateKey())
const visibleTasks = computed(() => activeView.value === 'reports' ? [] : filterTasks(
  appData.value.tasks,
  activeView.value,
  today.value,
  searchQuery.value,
  appData.value.lists,
))
const selectedTask = computed(() => appData.value.tasks.find((task) => task.id === selectedId.value) ?? null)
const totalOpen = computed(() => appData.value.tasks.filter((task) => task.status === 0).length)
const activeList = computed(() => activeView.value.startsWith('list:')
  ? appData.value.lists.find((list) => list.id === Number(activeView.value.slice(5))) ?? null
  : null)
const viewTitle = computed(() => {
  if (searchQuery.value.trim()) return '搜索结果'
  if (activeView.value === 'reports') return '日报 / 周报'
  if (activeView.value === 'today') return '今天'
  if (activeView.value === 'inbox') return '收集箱'
  if (activeView.value === 'planned') return '计划'
  if (activeView.value === 'completed') return '已完成'
  return activeList.value?.name ?? '我的清单'
})
const viewSubtitle = computed(() => {
  if (searchQuery.value.trim()) return `在所有任务中搜索“${searchQuery.value.trim()}”`
  if (activeView.value === 'reports') return '日报逐条记录，周报自动汇总本周日报'
  if (activeView.value === 'today') return dateLabel(new Date())
  if (activeView.value === 'inbox') return '还没有安排日期的任务'
  if (activeView.value === 'planned') return '为接下来留一点计划'
  if (activeView.value === 'completed') return '已经完成的每一件事'
  return '属于这个清单的任务'
})
const nextWorkReminder = computed(() => nextReminder(workSettings.value, reminderNow.value))
const reminderCountdown = computed(() => nextWorkReminder.value
  ? formatCountdown(nextWorkReminder.value.at, reminderNow.value)
  : '')
const reminderTitle = computed(() => nextWorkReminder.value?.kind === 'offWork' ? '距离下班' : '距离午休')

function dateLabel(date: Date): string {
  return new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric', weekday: 'long' }).format(date)
}

function taskCount(view: TaskView): number {
  return countTasks(appData.value.tasks, view, today.value)
}

function listCount(list: TodoList): number {
  return appData.value.tasks.filter((task) => task.status === 0 && task.list_id === list.id).length
}

function reportForTask(task: TodoTask): WorkReportEntry | undefined {
  return workReports.value.entries
    .filter((entry) => entry.task_id === task.id && entry.date === today.value)
    .sort((a, b) => b.updated_at.localeCompare(a.updated_at))[0]
}

function isWorkTask(task: TodoTask): boolean {
  return appData.value.lists.find((list) => list.id === task.list_id)?.name === '工作'
}

function getListName(id: number | null): string | undefined {
  return appData.value.lists.find((list) => list.id === id)?.name
}

function defaultWorkListId(data: AppData = appData.value): string {
  const workList = data.lists.find((list) => list.name === '工作')
  return workList ? String(workList.id) : ''
}

function flash(message: string): void {
  toast.value = message
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => { toast.value = '' }, 2800)
}

async function loadData(): Promise<void> {
  try {
    appData.value = await getAppData()
    if (!addListId.value || !appData.value.lists.some((list) => String(list.id) === addListId.value)) {
      addListId.value = defaultWorkListId(appData.value)
    }
    workSettings.value = await getWorkSettings()
    workReports.value = await getWorkReports()
    const shortcut = await getShortcutInfo()
    shortcutLabel.value = shortcut.label
    shortcutRegistered.value = shortcut.registered
    if (selectedId.value && !appData.value.tasks.some((task) => task.id === selectedId.value)) selectedId.value = null
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  } finally {
    loading.value = false
  }
}

function cloneWorkSettings(settings: WorkSettings): WorkSettings {
  return {
    lunch: { ...settings.lunch },
    offWork: { ...settings.offWork },
  }
}

function openSettings(): void {
  settingsDraft.value = cloneWorkSettings(workSettings.value)
  settingsOpen.value = true
}

function validateWorkSettings(settings: WorkSettings): string | null {
  for (const reminder of [settings.lunch, settings.offWork]) {
    if (!reminder.enabled) continue
    if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(reminder.time)) return '请输入有效的提醒时间'
    if (reminder.repeat === 'once' && !reminder.date) return '一次性提醒需要选择日期'
  }
  return null
}

function changeReminderRepeat(kind: 'lunch' | 'offWork'): void {
  const reminder = settingsDraft.value[kind]
  if (reminder.repeat === 'daily') reminder.date = null
  else if (!reminder.date) reminder.date = today.value
}

async function saveSettings(): Promise<void> {
  const error = validateWorkSettings(settingsDraft.value)
  if (error) {
    flash(error)
    return
  }
  try {
    await saveWorkSettings(settingsDraft.value)
    workSettings.value = cloneWorkSettings(settingsDraft.value)
    settingsOpen.value = false
    flash('工作时间提醒已保存')
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

function createReportId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID()
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`
}

async function persistReports(entries: WorkReportEntry[]): Promise<void> {
  const next = { entries }
  await saveWorkReports(next)
  workReports.value = next
}

async function saveDailyReport(content: string, progress: number, date: string): Promise<void> {
  const now = new Date().toISOString()
  const entry: WorkReportEntry = {
    id: createReportId(), date, content: content.trim(), progress: Math.max(0, Math.min(100, Math.round(progress))),
    task_id: null, created_at: now, updated_at: now,
  }
  try {
    await persistReports([entry, ...workReports.value.entries])
    flash('日报已保存')
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

async function updateReportProgress(id: string, progress: number): Promise<void> {
  const nextProgress = Math.max(0, Math.min(100, Math.round(progress)))
  const entries = workReports.value.entries.map((entry) => entry.id === id
    ? { ...entry, progress: nextProgress, updated_at: new Date().toISOString() }
    : entry)
  try {
    await persistReports(entries)
    flash('日报进度已更新')
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

function requestTaskReportSync(id: number): void {
  const task = appData.value.tasks.find((item) => item.id === id)
  if (!task) return
  if (!isWorkTask(task)) {
    flash('生活和其他清单任务不会同步到工作日报')
    return
  }
  if (reportForTask(task)) {
    flash('这条任务今天已经同步到日报')
    return
  }
  if (task.status === 1) {
    void saveTaskReport(task, 100)
    return
  }
  reportSyncTask.value = task
  reportSyncProgress.value = 0
  reportSyncOpen.value = true
}

async function saveTaskReport(task: TodoTask, progress: number): Promise<void> {
  const now = new Date().toISOString()
  const entry: WorkReportEntry = {
    id: createReportId(), date: today.value, content: task.title, progress: Math.max(0, Math.min(100, Math.round(progress))),
    task_id: task.id, created_at: now, updated_at: now,
  }
  try {
    await persistReports([entry, ...workReports.value.entries])
    reportSyncOpen.value = false
    reportSyncTask.value = null
    flash(entry.progress === 100 ? '已同步到日报，进度自动记为 100%' : '已同步到日报')
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

function confirmTaskReportSync(): void {
  if (!reportSyncTask.value) return
  void saveTaskReport(reportSyncTask.value, reportSyncProgress.value)
}

function cancelTaskReportSync(): void {
  reportSyncOpen.value = false
  reportSyncTask.value = null
}

async function setCelebrationWindow(fullscreen: boolean): Promise<void> {
  if (!nativeApp || isQuickAdd) return
  const window = getCurrentWindow()
  if (fullscreen) {
    await window.unminimize()
    await window.show()
  }
  await window.setFullscreen(fullscreen)
  await window.setAlwaysOnTop(fullscreen)
  await window.setFocus()
}

async function showCelebration(kind: ReminderKind): Promise<void> {
  celebration.value = kind
  try {
    await setCelebrationWindow(true)
  } catch (error) {
    flash(`全屏庆祝启动失败：${error instanceof Error ? error.message : String(error)}`)
  }
}

async function dismissCelebration(): Promise<void> {
  if (!celebration.value) return
  celebration.value = null
  try {
    await setCelebrationWindow(false)
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

async function triggerReminder(kind: ReminderKind): Promise<void> {
  const reminder = workSettings.value[kind]
  if (!reminder.enabled) return
  if (reminder.repeat === 'once') {
    const updated: WorkSettings = {
      ...workSettings.value,
      [kind]: { ...reminder, enabled: false, completed: true },
    }
    try {
      await saveWorkSettings(updated)
      workSettings.value = updated
    } catch (error) {
      flash(error instanceof Error ? error.message : String(error))
    }
  }
  await showCelebration(kind)
}

async function tickReminders(): Promise<void> {
  if (reminderTickBusy) return
  reminderTickBusy = true
  try {
    const now = new Date()
    const previous = previousReminderTime
    reminderNow.value = now
    previousReminderTime = now
    if (!previous) return
    for (const kind of ['lunch', 'offWork'] as ReminderKind[]) {
      const reminder = workSettings.value[kind]
      if (!crossedOccurrence(reminder, previous, now)) continue
      const key = `${kind}:${now.getFullYear()}-${now.getMonth()}-${now.getDate()}`
      if (triggeredReminders.has(key)) continue
      triggeredReminders.add(key)
      await triggerReminder(kind)
    }
  } finally {
    reminderTickBusy = false
  }
}

function handleGlobalKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && celebration.value) void dismissCelebration()
}

async function addTask(): Promise<void> {
  if (!addTitle.value.trim()) {
    addInput.value?.focus()
    return
  }
  const draft: TaskDraft = {
    title: addTitle.value.trim(),
    due_date: addDate.value || (activeView.value === 'today' ? today.value : null),
    due_time: (addDate.value || activeView.value === 'today') ? (addTime.value || null) : null,
    list_id: addListId.value ? Number(addListId.value) : (activeList.value?.id ?? null),
    priority: addPriority.value ? 1 : 0,
  }
  try {
    await createTask(draft)
    addTitle.value = ''
    addDate.value = today.value
    addTime.value = '18:00'
    addListId.value = defaultWorkListId()
    addPriority.value = false
    addOptionsOpen.value = false
    await loadData()
    addInput.value?.focus()
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

async function completeTask(id: number): Promise<void> {
  try {
    const updated = await toggleTask(id)
    appData.value.tasks = appData.value.tasks.map((task) => task.id === id ? updated : task)
    if (updated.status === 1) flash('做得好，这件事完成了')
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

async function saveTask(id: number, changes: TaskChanges): Promise<void> {
  const task = appData.value.tasks.find((item) => item.id === id)
  if (!task) return
  try {
    const updated = await updateTask(id, changes, task)
    appData.value.tasks = appData.value.tasks.map((item) => item.id === id ? updated : item)
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

async function removeTask(id: number): Promise<void> {
  try {
    await deleteTask(id)
    appData.value.tasks = appData.value.tasks.filter((task) => task.id !== id)
    selectedId.value = null
    flash('任务已删除')
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

function moveTask(targetId: number): void {
  if (draggingId.value === null || draggingId.value === targetId) return
  const ids = visibleTasks.value.map((task) => task.id)
  const from = ids.indexOf(draggingId.value)
  const to = ids.indexOf(targetId)
  if (from < 0 || to < 0) return
  ids.splice(to, 0, ...ids.splice(from, 1))
  const order = new Map(ids.map((id, index) => [id, index]))
  appData.value.tasks = appData.value.tasks.map((task) => order.has(task.id)
    ? { ...task, sort_order: order.get(task.id)! }
    : task)
  void reorderTasks(ids).catch((error) => flash(error instanceof Error ? error.message : String(error)))
}

async function addList(): Promise<void> {
  const name = newListName.value.trim()
  if (!name) return
  try {
    const list = await createList(name)
    appData.value.lists = [...appData.value.lists, list]
    newListName.value = ''
    newListOpen.value = false
    activeView.value = `list:${list.id}`
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

async function manageList(list: TodoList, action: 'rename' | 'delete'): Promise<void> {
  listMenuId.value = null
  if (action === 'rename') {
    const name = window.prompt('清单的新名称', list.name)?.trim()
    if (!name || name === list.name) return
    try {
      const updated = await renameList(list.id, name)
      appData.value.lists = appData.value.lists.map((item) => item.id === list.id ? updated : item)
    } catch (error) {
      flash(error instanceof Error ? error.message : String(error))
    }
    return
  }
  const hasTasks = appData.value.tasks.some((task) => task.list_id === list.id)
  const message = hasTasks
    ? `删除“${list.name}”清单？其中的任务会移到收集箱。`
    : `删除“${list.name}”清单？`
  if (!window.confirm(message)) return
  try {
    await deleteList(list.id)
    appData.value.lists = appData.value.lists.filter((item) => item.id !== list.id)
    appData.value.tasks = appData.value.tasks.map((task) => task.list_id === list.id ? { ...task, list_id: null } : task)
    if (activeView.value === `list:${list.id}`) activeView.value = 'inbox'
    flash('清单已删除，任务仍然保留')
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

function selectView(view: AppView): void {
  activeView.value = view
  selectedId.value = null
  searchQuery.value = ''
  listMenuId.value = null
}

function beginSearch(): void {
  searchOpen.value = true
  void nextTick(() => document.querySelector<HTMLInputElement>('.search-input')?.focus())
}

function closeSearch(): void {
  searchQuery.value = ''
  searchOpen.value = false
}

async function changeShortcut(label: string): Promise<void> {
  try {
    const info = await setShortcut(label)
    shortcutLabel.value = info.label
    shortcutRegistered.value = info.registered
    flash(`快速记录快捷键已设为 ${info.label}`)
  } catch (error) {
    flash(error instanceof Error ? error.message : String(error))
  }
}

async function checkForUpdates(): Promise<void> {
  if (checkingForUpdates.value) return
  checkingForUpdates.value = true
  updateProgress.value = null
  updateStatus.value = '正在检查更新…'
  try {
    const update = await check()
    if (!update) {
      updateStatus.value = '当前已是最新版本'
      return
    }
    let downloadedBytes = 0
    let totalBytes = 0
    updateStatus.value = `发现新版本 ${update.version}，准备下载…`
    await update.downloadAndInstall((event: DownloadEvent) => {
      if (event.event === 'Started') {
        totalBytes = event.data.contentLength ?? 0
        updateProgress.value = 0
        updateStatus.value = totalBytes > 0 ? `正在下载新版本 ${update.version}…` : `正在下载新版本 ${update.version}（大小未知）…`
      } else if (event.event === 'Progress') {
        downloadedBytes += event.data.chunkLength
        if (totalBytes > 0) updateProgress.value = Math.min(99, Math.round((downloadedBytes / totalBytes) * 100))
      } else {
        updateProgress.value = 100
        updateStatus.value = '下载完成，正在安装…'
      }
    })
    updateStatus.value = '更新已安装，应用即将重新启动'
  } catch (error) {
    updateProgress.value = null
    updateStatus.value = `检查更新失败：${error instanceof Error ? error.message : String(error)}`
  } finally {
    checkingForUpdates.value = false
  }
}

onMounted(async () => {
  if (isQuickAdd) return
  await loadData()
  unlisten = await subscribeToChanges(() => { void loadData() })
  dayTimer = setInterval(() => { today.value = localDateKey() }, 60_000)
  reminderTimer = setInterval(() => { void tickReminders() }, 1000)
  window.addEventListener('keydown', handleGlobalKeydown)
})
onBeforeUnmount(() => {
  unlisten?.()
  if (dayTimer) clearInterval(dayTimer)
  if (reminderTimer) clearInterval(reminderTimer)
  if (toastTimer) clearTimeout(toastTimer)
  window.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <QuickAddWindow v-if="isQuickAdd" />
  <div v-else class="app-shell" @click.self="listMenuId = null">
    <aside class="sidebar">
      <div class="brand-row">
        <div class="brand-mark"><Check :size="18" stroke-width="2.5" /></div>
        <div class="brand-wordmark"><strong>TinyTodo</strong><span>留一点空间给重要的事</span></div>
      </div>

      <nav class="primary-nav" aria-label="任务视图">
        <button class="nav-item" :class="{ active: activeView === 'today' && !searchQuery }" type="button" @click="selectView('today')">
          <Sun :size="17" /><span>今天</span><span v-if="taskCount('today')" class="nav-count">{{ taskCount('today') }}</span>
        </button>
        <button class="nav-item" :class="{ active: activeView === 'inbox' && !searchQuery }" type="button" @click="selectView('inbox')">
          <Inbox :size="17" /><span>收集箱</span><span v-if="taskCount('inbox')" class="nav-count">{{ taskCount('inbox') }}</span>
        </button>
        <button class="nav-item" :class="{ active: activeView === 'planned' && !searchQuery }" type="button" @click="selectView('planned')">
          <CalendarDays :size="17" /><span>计划</span><span v-if="taskCount('planned')" class="nav-count">{{ taskCount('planned') }}</span>
        </button>
        <button class="nav-item" :class="{ active: activeView === 'completed' && !searchQuery }" type="button" @click="selectView('completed')">
          <CheckCheck :size="17" /><span>已完成</span><span v-if="taskCount('completed')" class="nav-count">{{ taskCount('completed') }}</span>
        </button>
      </nav>

      <div class="sidebar-divider"></div>
      <div class="list-heading"><span>我的清单</span><span>{{ appData.lists.length }}</span></div>
      <nav class="list-nav" aria-label="自定义清单">
        <div v-for="list in appData.lists" :key="list.id" class="list-nav-wrap">
          <button class="nav-item list-nav-item" :class="{ active: activeView === `list:${list.id}` && !searchQuery }" type="button" @click="selectView(`list:${list.id}`)">
            <span class="list-dot"></span><span class="list-name">{{ list.name }}</span><span v-if="listCount(list)" class="nav-count">{{ listCount(list) }}</span>
          </button>
          <button class="list-more icon-button" type="button" :aria-label="`管理${list.name}清单`" @click.stop="listMenuId = listMenuId === list.id ? null : list.id"><MoreHorizontal :size="16" /></button>
          <div v-if="listMenuId === list.id" class="list-menu" @click.stop>
            <button type="button" @click="manageList(list, 'rename')">重命名</button>
            <button class="danger" type="button" @click="manageList(list, 'delete')">删除清单</button>
          </div>
        </div>
        <button v-if="!newListOpen" class="new-list-button" type="button" @click="newListOpen = true"><ListPlus :size="16" /> 新建清单</button>
        <form v-else class="new-list-form" @submit.prevent="addList">
          <input v-model="newListName" autofocus maxlength="40" placeholder="清单名称" @keydown.esc="newListOpen = false" />
          <button class="icon-button" type="submit" aria-label="创建清单"><Check :size="15" /></button>
          <button class="icon-button" type="button" aria-label="取消" @click="newListOpen = false"><X :size="15" /></button>
        </form>
      </nav>

      <button class="reports-nav-item" :class="{ active: activeView === 'reports' }" type="button" @click="selectView('reports')">
        <FileText :size="16" /><span>日报 / 周报</span>
      </button>

      <div class="sidebar-bottom">
        <button class="work-settings-entry" type="button" @click="openSettings">
          <span class="work-settings-entry-icon"><Clock3 :size="15" /></span>
          <span><strong>设置工作时间</strong><small>午休 / 下班提醒</small></span>
          <ChevronRight :size="14" />
        </button>
        <div class="shortcut-hint" :title="shortcutRegistered ? `快速记录：${shortcutLabel}` : '快捷键都被占用；仍可在主窗口快速添加'">
          <Sparkles :size="14" /><span>快速记录</span>
          <template v-if="shortcutRegistered"><kbd v-for="part in shortcutLabel.split(' + ')" :key="part">{{ part }}</kbd></template>
          <span v-else class="shortcut-unavailable">未注册</span>
        </div>
        <div class="open-count"><span class="status-dot"></span>{{ totalOpen }} 件待处理</div>
      </div>
    </aside>

    <main class="main-area">
      <header class="topbar">
        <div class="breadcrumb"><span>我的任务</span><ChevronRight :size="14" /><strong>{{ viewTitle }}</strong></div>
        <div class="top-actions">
          <label v-if="searchOpen" class="search-field">
            <Search :size="15" />
            <input v-model="searchQuery" class="search-input" type="search" placeholder="搜索任务和备注…" autofocus />
            <button class="icon-button" type="button" aria-label="关闭搜索" @click="closeSearch"><X :size="15" /></button>
          </label>
          <button v-else class="icon-button search-trigger" type="button" aria-label="搜索任务" title="搜索任务" @click="beginSearch"><Search :size="17" /></button>
          <button class="settings-trigger" type="button" title="设置工作时间和应用选项" aria-label="设置" @click="openSettings"><Settings2 :size="16" /><span>设置</span></button>
          <div class="avatar">T</div>
        </div>
      </header>

      <div v-if="nextWorkReminder" class="work-reminder-bar" role="status">
        <div class="work-reminder-copy"><Sparkles :size="15" /><span>{{ reminderTitle }}</span></div>
        <strong>{{ reminderCountdown }}</strong>
        <span class="work-reminder-hint">{{ nextWorkReminder.kind === 'offWork' ? '今天也辛苦了' : '记得起来休息一下' }}</span>
      </div>

      <div class="content-scroll">
        <ReportsPage
          v-if="activeView === 'reports'"
          :entries="workReports.entries"
          :today="today"
          @save="saveDailyReport"
          @update-progress="updateReportProgress"
          @notify="flash"
        />
        <section v-else class="task-page">
          <div class="page-heading">
            <div>
              <p class="eyebrow">{{ activeView === 'today' && !searchQuery ? '保持轻盈，一次专注一件事' : 'TinyTodo · 本地清单' }}</p>
              <h1>{{ viewTitle }}</h1>
              <p class="page-subtitle">{{ viewSubtitle }}</p>
            </div>
            <span v-if="visibleTasks.length" class="task-total">{{ visibleTasks.length }} 件</span>
          </div>

          <form v-if="activeView !== 'completed' && !searchQuery" class="quick-add" @submit.prevent="addTask">
            <div class="quick-add-main">
              <Plus class="quick-plus" :size="19" />
              <input ref="addInput" v-model="addTitle" type="text" maxlength="300" placeholder="添加一个任务…按 Enter 创建" autocomplete="off" />
              <button v-if="addTitle.trim()" class="create-button" type="submit">添加</button>
              <button class="quick-options-button" :class="{ active: addOptionsOpen }" type="button" aria-label="日期和优先级选项" @click="addOptionsOpen = !addOptionsOpen"><CalendarDays :size="16" /></button>
            </div>
            <div v-if="addOptionsOpen" class="quick-add-options">
              <label><CalendarDays :size="14" /><input v-model="addDate" type="date" :min="today" /></label>
              <label><Clock3 :size="14" /><input v-model="addTime" type="time" :disabled="!addDate" /></label>
              <select v-model="addListId" aria-label="任务清单"><option value="">无清单</option><option v-for="list in appData.lists" :key="list.id" :value="String(list.id)">{{ list.name }}</option></select>
              <button class="priority-toggle" :class="{ active: addPriority }" type="button" @click="addPriority = !addPriority"><Sparkles :size="13" /> 高优先级</button>
            </div>
          </form>

          <div v-if="activeView === 'today' && !searchQuery && visibleTasks.some((task) => task.due_date && task.due_date < today)" class="overdue-note">
            <Clock3 :size="14" /> 有 {{ visibleTasks.filter((task) => task.due_date && task.due_date < today).length }} 件逾期任务，排在列表中
          </div>

          <div v-if="loading" class="loading-state"><span class="spinner"></span>正在打开你的清单…</div>
          <div v-else-if="visibleTasks.length" class="task-list" role="list">
            <TaskRow
              v-for="task in visibleTasks"
              :key="task.id"
              :task="task"
              :selected="selectedId === task.id"
              :list-name="activeView === 'completed' ? getListName(task.list_id) : undefined"
              :dragging="draggingId === task.id"
              :drop-target="dragTargetId === task.id"
              :report-sync-visible="!searchQuery && (activeView === 'today' || activeView === 'completed')"
              :report-eligible="isWorkTask(task)"
              :report-entry="reportForTask(task)"
              @toggle="completeTask"
              @select="selectedId = selectedId === $event ? null : $event"
              @drag-start="draggingId = $event"
              @drag-over="dragTargetId = $event"
              @drop="moveTask($event)"
              @drag-end="draggingId = null; dragTargetId = null"
              @sync-report="requestTaskReportSync"
            />
          </div>
          <div v-else-if="!loading" class="empty-state">
            <div class="empty-illustration"><Archive v-if="activeView === 'inbox'" :size="24" /><CheckCheck v-else-if="activeView === 'completed'" :size="24" /><Search v-else-if="searchQuery" :size="24" /><Sun v-else :size="24" /></div>
            <h2>{{ searchQuery ? '没有找到匹配的任务' : activeView === 'completed' ? '完成的任务会出现在这里' : activeView === 'today' ? '今天还没有安排' : activeView === 'inbox' ? '收集箱空空的' : '这里暂时没有任务' }}</h2>
            <p>{{ searchQuery ? '试试换个关键词' : activeView === 'completed' ? '每完成一件事，就勾选它吧。' : activeView === 'today' ? '先把要做的事记下来，再慢慢安排。' : '写下第一件要做的事，开始整理。' }}</p>
            <button v-if="activeView !== 'completed' && !searchQuery" class="empty-add" type="button" @click="addInput?.focus()"><Plus :size="15" /> 添加任务</button>
          </div>

          <div class="bottom-note"><span></span>你的任务保存在这台设备上</div>
        </section>
      </div>
    </main>

    <Transition name="detail-slide">
      <TaskDetails
        v-if="selectedTask"
        :key="selectedTask.id"
        :task="selectedTask"
        :lists="appData.lists"
        @close="selectedId = null"
        @update="saveTask(selectedTask.id, $event)"
        @delete="removeTask"
      />
    </Transition>

    <Transition name="toast"><div v-if="toast" class="toast-message"><Check :size="15" />{{ toast }}</div></Transition>
    <div v-if="newListOpen" class="modal-backdrop" @click.self="newListOpen = false">
      <form class="new-list-modal" @submit.prevent="addList">
        <div class="modal-icon"><ListPlus :size="19" /></div>
        <h2>新建清单</h2>
        <p>给一组相关任务取个名字。</p>
        <input v-model="newListName" autofocus maxlength="40" placeholder="例如：工作、学习、生活" />
        <div class="modal-actions"><button type="button" class="text-button" @click="newListOpen = false">取消</button><button class="modal-primary" type="submit" :disabled="!newListName.trim()">创建清单</button></div>
      </form>
    </div>
    <div v-if="settingsOpen" class="modal-backdrop" @click.self="settingsOpen = false">
      <section class="new-list-modal settings-modal">
        <div class="modal-icon"><Settings2 :size="18" /></div>
        <h2>设置</h2>
        <p>调整工作节奏提醒、快速记录快捷键，或检查 TinyTodo 更新。</p>
        <div class="work-settings-section">
          <div class="settings-section-heading"><strong>工作节奏</strong><span>到点提醒你休息和下班</span></div>
          <div v-for="item in ([['lunch', '午休提醒'], ['offWork', '下班提醒']] as const)" :key="item[0]" class="reminder-setting-card">
            <div class="reminder-setting-head">
              <label class="reminder-toggle"><input v-model="settingsDraft[item[0]].enabled" type="checkbox" /><span class="toggle-track"></span><strong>{{ item[1] }}</strong></label>
              <input v-model="settingsDraft[item[0]].time" class="reminder-time" type="time" :disabled="!settingsDraft[item[0]].enabled" />
            </div>
            <div class="reminder-setting-fields" :class="{ disabled: !settingsDraft[item[0]].enabled }">
              <label><span>重复</span><select v-model="settingsDraft[item[0]].repeat" :disabled="!settingsDraft[item[0]].enabled" @change="changeReminderRepeat(item[0])"><option value="daily">每天重复</option><option value="once">一次性</option></select></label>
              <label v-if="settingsDraft[item[0]].repeat === 'once'"><span>日期</span><input v-model="settingsDraft[item[0]].date" type="date" :min="today" :disabled="!settingsDraft[item[0]].enabled" /></label>
            </div>
          </div>
          <p class="settings-note">下班时间到达后，当前显示器会全屏撒花庆祝；点击屏幕或按 Esc 退出。</p>
        </div>
        <label class="settings-label" for="shortcut-select">快速记录</label>
        <select id="shortcut-select" class="settings-select" :value="shortcutLabel" @change="changeShortcut(($event.target as HTMLSelectElement).value)">
          <option value="Ctrl + Shift + Space">Ctrl + Shift + Space</option>
          <option value="Ctrl + Alt + Space">Ctrl + Alt + Space</option>
          <option value="Alt + Shift + T">Alt + Shift + T</option>
        </select>
        <p class="settings-note">快捷键若被其他程序占用，会保留当前设置。应用关闭时会收进系统托盘。</p>
        <div class="update-row">
          <div class="update-copy">
            <strong>软件更新</strong>
            <span v-if="updateStatus" role="status">{{ updateStatus }}</span>
            <span v-else>从 GitHub 检查 Windows 版本更新</span>
              <div v-if="updateProgress !== null" class="update-progress" role="progressbar" :aria-valuenow="updateProgress" aria-valuemin="0" aria-valuemax="100">
                <div class="update-progress-meta"><span>下载进度</span><b>{{ updateProgress }}%</b></div>
                <div class="update-progress-track"><i :style="{ width: `${updateProgress}%` }"></i></div>
              </div>
          </div>
          <button class="update-button" type="button" :disabled="checkingForUpdates" @click="checkForUpdates">
            {{ checkingForUpdates ? (updateProgress === null ? '检查中…' : '下载中…') : '检查更新' }}
          </button>
        </div>
        <div class="modal-actions"><button class="text-button" type="button" @click="settingsOpen = false">取消</button><button class="modal-primary" type="button" @click="saveSettings">保存设置</button></div>
      </section>
    </div>
    <div v-if="reportSyncOpen && reportSyncTask" class="modal-backdrop" @click.self="cancelTaskReportSync">
      <form class="new-list-modal report-sync-modal" @submit.prevent="confirmTaskReportSync">
        <div class="modal-icon"><FileText :size="18" /></div>
        <h2>同步到工作日报</h2>
        <p>{{ reportSyncTask.title }}</p>
        <label class="settings-label" for="report-sync-progress">当前进度</label>
        <div class="report-sync-input"><input id="report-sync-progress" v-model.number="reportSyncProgress" type="number" min="0" max="99" step="1" autofocus /><span>%</span></div>
        <p class="settings-note">任务尚未完成，请填写当前进度。完成任务会自动记录为 100%。</p>
        <div class="modal-actions"><button class="text-button" type="button" @click="cancelTaskReportSync">取消</button><button class="modal-primary" type="submit">同步日报</button></div>
      </form>
    </div>
    <div v-if="celebration" class="celebration-screen" role="dialog" aria-modal="true" @click="dismissCelebration">
      <div class="celebration-confetti" aria-hidden="true"><span v-for="n in 34" :key="n" :style="{ '--i': n }"></span></div>
      <div class="celebration-message">
        <Sparkles :size="42" stroke-width="1.4" />
        <p class="celebration-kicker">{{ celebration === 'offWork' ? '工作完成' : '休息一下' }}</p>
        <h2>{{ celebration === 'offWork' ? '下班啦，今天也辛苦了' : '午休时间到了' }}</h2>
        <p>点击任意位置关闭庆祝</p>
      </div>
    </div>
  </div>
</template>
