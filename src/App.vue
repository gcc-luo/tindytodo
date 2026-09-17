<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  Archive, CalendarDays, Check, CheckCheck, ChevronRight, Clock3,
  Inbox, ListPlus, MoreHorizontal, Plus, Search, Settings2, Sparkles, Sun, X,
} from 'lucide-vue-next'
import TaskDetails from './components/TaskDetails.vue'
import TaskRow from './components/TaskRow.vue'
import QuickAddWindow from './components/QuickAddWindow.vue'
import {
  createList, createTask, deleteList, deleteTask, getAppData, getShortcutInfo, reorderTasks, setShortcut,
  renameList, subscribeToChanges, toggleTask, updateTask,
} from './api'
import { countTasks, filterTasks, localDateKey } from './lib/taskFilters'
import type { AppData, TaskDraft, TaskView, TodoList, TaskChanges } from './types'

const isQuickAdd = new URLSearchParams(window.location.search).has('quickAdd')
const appData = ref<AppData>({ tasks: [], lists: [] })
const loading = ref(true)
const activeView = ref<TaskView>('today')
const selectedId = ref<number | null>(null)
const searchOpen = ref(false)
const searchQuery = ref('')
const toast = ref('')
const shortcutLabel = ref('Ctrl + Alt + Space')
const shortcutRegistered = ref(true)
const settingsOpen = ref(false)
const addTitle = ref('')
const addOptionsOpen = ref(false)
const addDate = ref('')
const addTime = ref('')
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

const today = ref(localDateKey())
const visibleTasks = computed(() => filterTasks(
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
  if (activeView.value === 'today') return '今天'
  if (activeView.value === 'inbox') return '收集箱'
  if (activeView.value === 'planned') return '计划'
  if (activeView.value === 'completed') return '已完成'
  return activeList.value?.name ?? '我的清单'
})
const viewSubtitle = computed(() => {
  if (searchQuery.value.trim()) return `在所有任务中搜索“${searchQuery.value.trim()}”`
  if (activeView.value === 'today') return dateLabel(new Date())
  if (activeView.value === 'inbox') return '还没有安排日期的任务'
  if (activeView.value === 'planned') return '为接下来留一点计划'
  if (activeView.value === 'completed') return '已经完成的每一件事'
  return '属于这个清单的任务'
})

function dateLabel(date: Date): string {
  return new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric', weekday: 'long' }).format(date)
}

function taskCount(view: TaskView): number {
  return countTasks(appData.value.tasks, view, today.value)
}

function listCount(list: TodoList): number {
  return appData.value.tasks.filter((task) => task.status === 0 && task.list_id === list.id).length
}

function getListName(id: number | null): string | undefined {
  return appData.value.lists.find((list) => list.id === id)?.name
}

function flash(message: string): void {
  toast.value = message
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => { toast.value = '' }, 2800)
}

async function loadData(): Promise<void> {
  try {
    appData.value = await getAppData()
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
    addDate.value = ''
    addTime.value = ''
    addListId.value = ''
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

function selectView(view: TaskView): void {
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

onMounted(async () => {
  await loadData()
  unlisten = await subscribeToChanges(() => { void loadData() })
  dayTimer = setInterval(() => { today.value = localDateKey() }, 60_000)
})
onBeforeUnmount(() => {
  unlisten?.()
  if (dayTimer) clearInterval(dayTimer)
  if (toastTimer) clearTimeout(toastTimer)
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

      <div class="sidebar-bottom">
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
          <button class="help-button" type="button" title="设置" aria-label="设置" @click="settingsOpen = true"><Settings2 :size="16" /></button>
          <div class="avatar">T</div>
        </div>
      </header>

      <div class="content-scroll">
        <section class="task-page">
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
              @toggle="completeTask"
              @select="selectedId = selectedId === $event ? null : $event"
              @drag-start="draggingId = $event"
              @drag-over="dragTargetId = $event"
              @drop="moveTask($event)"
              @drag-end="draggingId = null; dragTargetId = null"
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
        <p>调整快速记录的全局快捷键。</p>
        <label class="settings-label" for="shortcut-select">快速记录</label>
        <select id="shortcut-select" class="settings-select" :value="shortcutLabel" @change="changeShortcut(($event.target as HTMLSelectElement).value)">
          <option value="Ctrl + Shift + Space">Ctrl + Shift + Space</option>
          <option value="Ctrl + Alt + Space">Ctrl + Alt + Space</option>
          <option value="Alt + Shift + T">Alt + Shift + T</option>
        </select>
        <p class="settings-note">快捷键若被其他程序占用，会保留当前设置。应用关闭时会收进系统托盘。</p>
        <div class="modal-actions"><button class="modal-primary" type="button" @click="settingsOpen = false">完成</button></div>
      </section>
    </div>
  </div>
</template>
