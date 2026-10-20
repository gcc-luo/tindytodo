<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import {
  CalendarDays, ChevronDown, ChevronLeft, ChevronRight, CirclePlus, NotebookPen,
  Plus, RotateCcw, Search, Tag, Trash2, X,
} from 'lucide-vue-next'
import DiaryEditor, { type DiarySnapshot } from './DiaryEditor.vue'
import {
  createDiaryEntry, createDiaryTag, deleteDiaryTag, purgeDiaryEntry,
  renameDiaryTag, restoreDiaryEntry, saveDiaryEntry, trashDiaryEntry,
} from '../api'
import { createDiaryAutosave, type DiarySaveStatus } from '../lib/diaryAutosave'
import {
  diaryEntryPreview, filterDiaryEntries, groupEntriesByDate, localDateKey,
  monthCalendarCells, monthEntryCounts,
} from '../lib/diaryDates'
import { textFromDiaryHtml } from '../lib/diaryEditor'
import type { DiaryData, DiaryEntry } from '../types'

const props = defineProps<{ data: DiaryData; initialDate: string }>()
const emit = defineEmits<{ switchWorkspace: [workspace: 'tasks'] }>()

const entries = ref([...props.data.entries])
const tags = ref([...props.data.tags])
const selectedDate = ref(props.initialDate || localDateKey())
const displayMonth = ref(dateFromKey(selectedDate.value))
const selectedEntryId = ref<number | null>(null)
const selectedTagId = ref<number | null>(null)
const searchQuery = ref('')
const trashView = ref(false)
const expandedYears = ref(new Set([selectedDate.value.slice(0, 4)]))
const expandedMonths = ref(new Set([selectedDate.value.slice(0, 7)]))
const expandedDates = ref(new Set([selectedDate.value]))
const saveStatus = ref<DiarySaveStatus>('idle')
const saveError = ref('')
const notice = ref('')
const loading = ref(false)
let autosave: ReturnType<typeof createDiaryAutosave<DiarySnapshot>> | null = null
let noticeTimer: ReturnType<typeof setTimeout> | undefined

const todayKey = computed(() => localDateKey())
const selectedEntry = computed(() => entries.value.find((entry) => entry.id === selectedEntryId.value) ?? null)
const allActiveEntries = computed(() => entries.value.filter((entry) => entry.deleted_at === null))
const filteredEntries = computed(() => filterDiaryEntries(entries.value, {
  date: trashView.value ? null : selectedDate.value,
  tagId: trashView.value ? null : selectedTagId.value,
  query: searchQuery.value,
  trash: trashView.value,
}))
const treeEntries = computed(() => filterDiaryEntries(entries.value, {
  date: null, tagId: selectedTagId.value, query: searchQuery.value, trash: false,
}))
const selectedDateEntries = computed(() => filteredEntries.value.filter((entry) => entry.entry_date === selectedDate.value))
const countsByDate = computed(() => monthEntryCounts(entries.value, monthKey(displayMonth.value)))
const calendarCells = computed(() => monthCalendarCells(displayMonth.value))
const monthTitle = computed(() => new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: 'long' }).format(displayMonth.value))
const weekDays = ['日', '一', '二', '三', '四', '五', '六']
const allEntryCount = computed(() => allActiveEntries.value.length)
const tagCounts = computed(() => new Map(tags.value.map((tag) => [tag.id, allActiveEntries.value.filter((entry) => entry.tag_ids.includes(tag.id)).length])))
const dateTree = computed(() => {
  const byDate = groupEntriesByDate(treeEntries.value)
  const years = new Map<string, Map<string, Array<{ date: string; entries: DiaryEntry[] }>>>()
  for (const [date, dayEntries] of byDate) {
    const year = date.slice(0, 4)
    const month = date.slice(0, 7)
    const months = years.get(year) ?? new Map<string, Array<{ date: string; entries: DiaryEntry[] }>>()
    const dates = months.get(month) ?? []
    dates.push({ date, entries: dayEntries })
    months.set(month, dates)
    years.set(year, months)
  }
  return [...years.entries()].sort(([a], [b]) => b.localeCompare(a)).map(([year, months]) => ({
    year,
    months: [...months.entries()].sort(([a], [b]) => b.localeCompare(a)).map(([month, dates]) => ({ month, dates })),
  }))
})

watch(() => props.data, (data) => {
  entries.value = [...data.entries]
  tags.value = [...data.tags]
}, { deep: true })

function dateFromKey(key: string): Date {
  const [year, month, day] = key.split('-').map(Number)
  return new Date(year || new Date().getFullYear(), (month || 1) - 1, day || 1, 12)
}

function monthKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`
}

function showNotice(message: string) {
  notice.value = message
  if (noticeTimer) clearTimeout(noticeTimer)
  noticeTimer = setTimeout(() => { notice.value = '' }, 2600)
}

function messageForError(error: unknown): string {
  return error instanceof Error ? error.message : '操作失败，请重试'
}

async function flushBeforeNavigation(): Promise<boolean> {
  if (!autosave) return true
  try {
    await autosave.flush()
    saveError.value = ''
    return true
  } catch (error) {
    saveError.value = `保存失败：${messageForError(error)}。当前日记仍保留在此处，请重试后再切换。`
    return false
  }
}

function attachAutosave(entryId: number) {
  autosave?.dispose()
  saveError.value = ''
  saveStatus.value = 'idle'
  autosave = createDiaryAutosave(async (snapshot) => {
    const updated = await saveDiaryEntry(entryId, snapshot.title, snapshot.content_html, snapshot.tag_ids)
    entries.value = entries.value.map((entry) => entry.id === entryId ? updated : entry)
  }, 450, (status, error) => {
    saveStatus.value = status
    saveError.value = status === 'error' ? messageForError(error) : ''
  })
}

function receiveSnapshot(snapshot: DiarySnapshot) {
  const entry = selectedEntry.value
  if (!entry || trashView.value) return
  entries.value = entries.value.map((item) => item.id === entry.id
    ? { ...item, title: snapshot.title, content_html: snapshot.content_html, tag_ids: snapshot.tag_ids }
    : item)
  autosave?.schedule(snapshot)
}

async function selectEntry(entry: DiaryEntry) {
  if (selectedEntryId.value === entry.id) return
  if (!await flushBeforeNavigation()) return
  autosave?.dispose()
  autosave = null
  selectedEntryId.value = entry.id
  selectedDate.value = entry.entry_date
  displayMonth.value = dateFromKey(entry.entry_date)
  trashView.value = entry.deleted_at !== null
  expandDate(entry.entry_date)
  if (entry.deleted_at === null) attachAutosave(entry.id)
}

function expandDate(date: string) {
  expandedYears.value = new Set([...expandedYears.value, date.slice(0, 4)])
  expandedMonths.value = new Set([...expandedMonths.value, date.slice(0, 7)])
  expandedDates.value = new Set([...expandedDates.value, date])
}

async function selectDate(date: string) {
  if (date === selectedDate.value && selectedEntryId.value === null && monthKey(displayMonth.value) === date.slice(0, 7)) return
  if (!await flushBeforeNavigation()) return
  autosave?.dispose()
  autosave = null
  selectedEntryId.value = null
  trashView.value = false
  selectedDate.value = date
  displayMonth.value = dateFromKey(date)
  expandDate(date)
}

async function createEntry() {
  if (!await flushBeforeNavigation()) return
  try {
    const entry = await createDiaryEntry(selectedDate.value || todayKey.value)
    entries.value = [...entries.value, entry]
    selectedEntryId.value = entry.id
    selectedDate.value = entry.entry_date
    trashView.value = false
    attachAutosave(entry.id)
    expandDate(entry.entry_date)
    await nextTick()
    showNotice('新日记已创建，开始记录吧')
  } catch (error) {
    showNotice(messageForError(error))
  }
}

async function moveToTrash() {
  const entry = selectedEntry.value
  if (!entry || !await flushBeforeNavigation()) return
  try {
    await trashDiaryEntry(entry.id)
    entries.value = entries.value.map((item) => item.id === entry.id ? { ...item, deleted_at: new Date().toISOString() } : item)
    autosave?.dispose()
    autosave = null
    selectedEntryId.value = null
    showNotice('日记已移入回收站')
  } catch (error) {
    showNotice(messageForError(error))
  }
}

async function restoreEntry(entry: DiaryEntry) {
  try {
    await restoreDiaryEntry(entry.id)
    entries.value = entries.value.map((item) => item.id === entry.id ? { ...item, deleted_at: null } : item)
    if (selectedEntryId.value === entry.id) {
      trashView.value = false
      attachAutosave(entry.id)
    }
    showNotice('日记已恢复')
  } catch (error) {
    showNotice(messageForError(error))
  }
}

async function permanentlyDelete(entry: DiaryEntry) {
  if (!window.confirm(`彻底删除“${diaryEntryPreview(entry)}”？此操作无法撤销。`)) return
  try {
    await purgeDiaryEntry(entry.id)
    entries.value = entries.value.filter((item) => item.id !== entry.id)
    if (selectedEntryId.value === entry.id) selectedEntryId.value = null
    showNotice('日记已彻底删除')
  } catch (error) {
    showNotice(messageForError(error))
  }
}

async function toggleTrash() {
  if (!await flushBeforeNavigation()) return
  autosave?.dispose()
  autosave = null
  selectedEntryId.value = null
  trashView.value = !trashView.value
  selectedTagId.value = null
}

async function switchToTasks() {
  if (await flushBeforeNavigation()) emit('switchWorkspace', 'tasks')
}

function changeMonth(amount: number) {
  const next = new Date(displayMonth.value)
  next.setDate(1)
  next.setMonth(next.getMonth() + amount)
  displayMonth.value = next
}

function toggleSet(setRef: typeof expandedYears, value: string) {
  const next = new Set(setRef.value)
  if (next.has(value)) next.delete(value)
  else next.add(value)
  setRef.value = next
}

async function addTag() {
  const name = window.prompt('新标签名称')?.trim()
  if (!name) return
  try {
    const tag = await createDiaryTag(name)
    tags.value = [...tags.value, tag]
    selectedTagId.value = tag.id
    showNotice('标签已创建')
  } catch (error) {
    showNotice(messageForError(error))
  }
}

async function editTag(id: number, action: 'rename' | 'delete') {
  const tag = tags.value.find((item) => item.id === id)
  if (!tag) return
  try {
    if (action === 'rename') {
      const name = window.prompt('重命名标签', tag.name)?.trim()
      if (!name) return
      const updated = await renameDiaryTag(id, name)
      tags.value = tags.value.map((item) => item.id === id ? updated : item)
      showNotice('标签名称已更新')
    } else {
      if (!window.confirm(`删除标签“${tag.name}”？日记内容会保留。`)) return
      if (selectedEntry.value?.tag_ids.includes(id) && !await flushBeforeNavigation()) return
      await deleteDiaryTag(id)
      tags.value = tags.value.filter((item) => item.id !== id)
      entries.value = entries.value.map((entry) => ({ ...entry, tag_ids: entry.tag_ids.filter((tagId) => tagId !== id) }))
      if (selectedTagId.value === id) selectedTagId.value = null
      showNotice('标签已删除')
    }
  } catch (error) {
    showNotice(messageForError(error))
  }
}

function selectTag(id: number | null) {
  selectedTagId.value = selectedTagId.value === id ? null : id
}

function toggleYear(year: string) { toggleSet(expandedYears, year) }
function toggleMonth(month: string) { toggleSet(expandedMonths, month) }
async function toggleDate(date: string) {
  toggleSet(expandedDates, date)
  await selectDate(date)
}

function prettyMonth(month: string) {
  return `${Number(month.slice(5, 7))} 月`
}

function dayLabel(date: string) {
  return `${Number(date.slice(8, 10))} 日`
}

function dateSummary(entry: DiaryEntry) {
  return textFromDiaryHtml(entry.content_html).slice(0, 96)
}

async function retrySave() {
  await flushBeforeNavigation()
}

</script>

<template>
  <div class="diary-workspace">
    <aside class="diary-sidebar">
      <div class="diary-sidebar-brand">
        <div class="diary-brand-icon"><NotebookPen :size="18" /></div>
        <div><strong>我的日记</strong><small>收集日常里的微光</small></div>
        <button class="diary-back-button" type="button" @click="switchToTasks">待办</button>
      </div>

      <label class="diary-search">
        <Search :size="16" />
        <input v-model="searchQuery" :placeholder="trashView ? '搜索回收站' : '搜索日记'" aria-label="搜索日记标题和正文">
        <button v-if="searchQuery" type="button" aria-label="清除搜索" @click="searchQuery = ''"><X :size="13" /></button>
      </label>

      <section class="diary-calendar-panel" aria-label="日记日历">
        <div class="diary-calendar-heading">
          <button type="button" aria-label="上个月" @click="changeMonth(-1)"><ChevronLeft :size="16" /></button>
          <strong>{{ monthTitle }}</strong>
          <button type="button" aria-label="下个月" @click="changeMonth(1)"><ChevronRight :size="16" /></button>
        </div>
        <div class="diary-calendar-grid diary-calendar-weekdays"><span v-for="day in weekDays" :key="day">{{ day }}</span></div>
        <div class="diary-calendar-grid diary-calendar-days">
          <button
            v-for="cell in calendarCells"
            :key="localDateKey(cell)"
            type="button"
            :class="{
              'is-adjacent': cell.getMonth() !== displayMonth.getMonth(),
              'is-today': localDateKey(cell) === todayKey,
              'is-selected': localDateKey(cell) === selectedDate,
              'has-entry': (countsByDate.get(localDateKey(cell)) ?? 0) > 0,
            }"
            :aria-label="`${localDateKey(cell)}${countsByDate.get(localDateKey(cell)) ? `，${countsByDate.get(localDateKey(cell))} 篇日记` : ''}`"
            @click="selectDate(localDateKey(cell))"
          >
            <span>{{ cell.getDate() }}</span>
            <i v-if="countsByDate.has(localDateKey(cell))" :aria-label="`${countsByDate.get(localDateKey(cell))} 篇日记`">{{ countsByDate.get(localDateKey(cell)) }}</i>
          </button>
        </div>
      </section>

      <section class="diary-tags-panel">
        <div class="diary-section-heading"><span>标签</span><button type="button" aria-label="新建标签" title="新建标签" @click="addTag"><Plus :size="14" /></button></div>
        <button class="diary-tag-row" :class="{ active: selectedTagId === null && !trashView }" type="button" @click="selectTag(null)">
          <Tag :size="15" /><span>全部日记</span><small>{{ allEntryCount }}</small>
        </button>
        <div v-for="tag in tags" :key="tag.id" class="diary-tag-row-wrap">
          <button class="diary-tag-row" :class="{ active: selectedTagId === tag.id && !trashView }" type="button" @click="selectTag(tag.id)">
            <Tag :size="15" /><span>{{ tag.name }}</span><small>{{ tagCounts.get(tag.id) ?? 0 }}</small>
          </button>
          <div class="diary-tag-actions">
            <button type="button" :aria-label="`重命名标签${tag.name}`" @click="editTag(tag.id, 'rename')">改名</button>
            <button type="button" :aria-label="`删除标签${tag.name}`" @click="editTag(tag.id, 'delete')">删除</button>
          </div>
        </div>
      </section>

      <section class="diary-tree-panel" :aria-label="trashView ? '回收站日记' : '按日期浏览日记'">
        <div class="diary-section-heading"><span>{{ trashView ? '回收站' : '我的日记' }}</span><small>{{ trashView ? filteredEntries.length : treeEntries.length }}</small></div>
        <template v-if="trashView">
          <article v-for="entry in filteredEntries" :key="entry.id" class="diary-trash-row" :class="{ active: selectedEntryId === entry.id }">
            <button class="diary-trash-summary" type="button" @click="selectEntry(entry)">
              <strong>{{ diaryEntryPreview(entry) }}</strong><small>{{ entry.entry_date }}</small>
            </button>
            <button type="button" :aria-label="`恢复${diaryEntryPreview(entry)}`" title="恢复" @click="restoreEntry(entry)"><RotateCcw :size="14" /></button>
            <button type="button" :aria-label="`彻底删除${diaryEntryPreview(entry)}`" title="彻底删除" @click="permanentlyDelete(entry)"><Trash2 :size="14" /></button>
          </article>
          <p v-if="filteredEntries.length === 0" class="diary-tree-empty">回收站是空的</p>
        </template>
        <template v-else>
          <div v-for="year in dateTree" :key="year.year" class="diary-tree-year">
            <button class="diary-tree-toggle diary-tree-year-toggle" type="button" @click="toggleYear(year.year)">
              <ChevronDown :size="13" :class="{ collapsed: !expandedYears.has(year.year) }" /><CalendarDays :size="15" /><span>{{ year.year }} 年</span>
            </button>
            <div v-if="expandedYears.has(year.year)" class="diary-tree-branch">
              <div v-for="month in year.months" :key="month.month" class="diary-tree-month">
                <button class="diary-tree-toggle" type="button" @click="toggleMonth(month.month)">
                  <ChevronDown :size="12" :class="{ collapsed: !expandedMonths.has(month.month) }" /><CalendarDays :size="14" /><span>{{ prettyMonth(month.month) }}</span>
                </button>
                <div v-if="expandedMonths.has(month.month)" class="diary-tree-branch">
                  <div v-for="day in month.dates" :key="day.date" class="diary-tree-day">
                    <button class="diary-tree-toggle diary-tree-date" :class="{ selected: selectedDate === day.date }" type="button" @click="toggleDate(day.date)">
                      <ChevronDown :size="11" :class="{ collapsed: !expandedDates.has(day.date) }" /><span>{{ dayLabel(day.date) }}</span><small>{{ day.entries.length }}</small>
                    </button>
                    <div v-if="expandedDates.has(day.date)" class="diary-tree-entries">
                      <button v-for="entry in day.entries" :key="entry.id" class="diary-tree-entry" :class="{ active: selectedEntryId === entry.id }" type="button" @click="selectEntry(entry)">
                        <span>{{ diaryEntryPreview(entry) }}</span><small>{{ new Date(entry.created_at.replace(' ', 'T') + (entry.created_at.endsWith('Z') ? '' : 'Z')).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}</small>
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <p v-if="dateTree.length === 0" class="diary-tree-empty">还没有日记，写下第一篇吧</p>
        </template>
      </section>

      <div class="diary-sidebar-bottom">
        <button class="diary-new-entry-button" type="button" @click="createEntry"><CirclePlus :size="16" />写一篇日记</button>
        <button class="diary-trash-toggle" :class="{ active: trashView }" type="button" @click="toggleTrash"><Trash2 :size="15" /><span>回收站</span><small>{{ entries.filter((entry) => entry.deleted_at !== null).length }}</small></button>
      </div>
    </aside>

    <main class="diary-main-panel">
      <div v-if="notice" class="diary-notice" role="status">{{ notice }}</div>
      <div v-if="loading" class="diary-main-empty"><span class="spinner"></span>正在打开日记…</div>
      <template v-else-if="trashView">
        <header class="diary-list-heading"><div><p>回收站</p><h1>{{ selectedEntry ? diaryEntryPreview(selectedEntry) : '已删除的日记' }}</h1></div><button v-if="selectedEntry" type="button" @click="restoreEntry(selectedEntry)"><RotateCcw :size="15" />恢复这篇日记</button></header>
        <article v-if="selectedEntry" class="diary-trash-detail"><p>{{ selectedEntry.entry_date }}</p><h2>{{ selectedEntry.title || '无标题日记' }}</h2><p>{{ dateSummary(selectedEntry) || '这篇日记没有正文。' }}</p><button class="diary-danger-action" type="button" @click="permanentlyDelete(selectedEntry)"><Trash2 :size="14" />彻底删除</button></article>
        <div v-else class="diary-main-empty"><Trash2 :size="26" /><h2>选择一篇日记</h2><p>可在左侧恢复或彻底删除</p></div>
      </template>
      <DiaryEditor
        v-else-if="selectedEntry"
        :key="selectedEntry.id"
        :entry="selectedEntry"
        :tags="tags"
        :save-status="saveStatus"
        :save-error="saveError"
        @save="receiveSnapshot"
        @retry="retrySave"
        @trash="moveToTrash"
      />
      <section v-else class="diary-day-overview">
        <header class="diary-list-heading">
          <div><p>{{ selectedDate === todayKey ? '今天' : '按日期浏览' }}</p><h1>{{ selectedDate }}</h1></div>
          <button type="button" class="diary-heading-create" @click="createEntry"><Plus :size="15" />新建日记</button>
        </header>
        <div v-if="selectedDateEntries.length" class="diary-day-entry-list">
          <button v-for="entry in selectedDateEntries" :key="entry.id" type="button" class="diary-day-entry-card" @click="selectEntry(entry)">
            <span class="diary-day-entry-time">{{ new Date(entry.created_at.replace(' ', 'T') + (entry.created_at.endsWith('Z') ? '' : 'Z')).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}</span>
            <strong>{{ diaryEntryPreview(entry) }}</strong>
            <span>{{ dateSummary(entry) }}</span>
            <small>{{ entry.tag_ids.map((id) => tags.find((tag) => tag.id === id)?.name).filter(Boolean).join(' · ') }}</small>
          </button>
        </div>
        <div v-else class="diary-main-empty"><div class="diary-empty-icon"><NotebookPen :size="25" /></div><h2>这一天还没有日记</h2><p>留下一段文字，记住今天的心情与故事。</p><button type="button" class="diary-heading-create" @click="createEntry"><Plus :size="15" />写下今天</button></div>
      </section>
    </main>
  </div>
</template>
