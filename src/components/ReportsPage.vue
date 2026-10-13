<script setup lang="ts">
import { computed, ref } from 'vue'
import { ClipboardCopy, FileText, Plus, Save, Sparkles } from 'lucide-vue-next'
import type { WorkReportEntry } from '../types'

const props = defineProps<{
  entries: WorkReportEntry[]
  today: string
}>()

const emit = defineEmits<{
  save: [content: string, progress: number, date: string]
  updateProgress: [id: string, progress: number]
  notify: [message: string]
}>()

const tab = ref<'daily' | 'weekly' | 'history'>('daily')
const content = ref('')
const progress = ref(0)

function dateValue(value: string): Date {
  const [year, month, day] = value.split('-').map(Number)
  return new Date(year, month - 1, day)
}

function dateKey(value: Date): string {
  const year = value.getFullYear()
  const month = String(value.getMonth() + 1).padStart(2, '0')
  const day = String(value.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

function displayDate(value: string): string {
  const [, month, day] = value.split('-')
  return `${Number(month)}月${Number(day)}日`
}

const todayEntries = computed(() => props.entries
  .filter((entry) => entry.date === props.today)
  .sort((a, b) => b.updated_at.localeCompare(a.updated_at)))

const weekStart = computed(() => {
  const date = dateValue(props.today)
  const day = date.getDay() || 7
  date.setDate(date.getDate() - day + 1)
  return date
})

const weekEnd = computed(() => {
  const date = new Date(weekStart.value)
  date.setDate(date.getDate() + 6)
  return date
})

const weekEntries = computed(() => props.entries
  .filter((entry) => {
    const date = dateValue(entry.date)
    return date >= weekStart.value && date <= weekEnd.value
  })
  .sort((a, b) => b.date.localeCompare(a.date) || b.updated_at.localeCompare(a.updated_at)))

const historyGroups = computed(() => {
  const groups = new Map<string, WorkReportEntry[]>()
  for (const entry of [...props.entries].sort((a, b) => b.date.localeCompare(a.date) || b.updated_at.localeCompare(a.updated_at))) {
    const group = groups.get(entry.date) ?? []
    group.push(entry)
    groups.set(entry.date, group)
  }
  return [...groups.entries()]
})

const averageProgress = computed(() => weekEntries.value.length
  ? Math.round(weekEntries.value.reduce((total, entry) => total + entry.progress, 0) / weekEntries.value.length)
  : 0)

function submit(): void {
  const value = content.value.trim()
  if (!value) {
    emit('notify', '请先填写日报内容')
    return
  }
  emit('save', value, clampProgress(progress.value), props.today)
  content.value = ''
  progress.value = 0
}

function clampProgress(value: number): number {
  return Math.max(0, Math.min(100, Math.round(Number.isFinite(value) ? value : 0)))
}

function changeProgress(entry: WorkReportEntry, value: string): void {
  emit('updateProgress', entry.id, clampProgress(Number(value)))
}

async function copyText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text)
    emit('notify', '日报已复制为多行文本')
  } catch {
    const textarea = document.createElement('textarea')
    textarea.value = text
    textarea.style.position = 'fixed'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)
    textarea.select()
    document.execCommand('copy')
    textarea.remove()
    emit('notify', '日报已复制为多行文本')
  }
}

function entriesText(entries: WorkReportEntry[]): string {
  return entries.map((entry) => `${entry.date}　${entry.content}（进度 ${entry.progress}%）`).join('\n')
}

function copyToday(): Promise<void> {
  return copyText(entriesText(todayEntries.value))
}

function copyWeek(): Promise<void> {
  const title = `本周周报（${dateKey(weekStart.value)} — ${dateKey(weekEnd.value)}）`
  return copyText([title, ...weekEntries.value.map((entry) => `${entry.date}　${entry.content}（进度 ${entry.progress}%）`)].join('\n'))
}
</script>

<template>
  <section class="reports-page">
    <div class="reports-heading">
      <div>
        <p class="eyebrow">保持轻盈，给今天留一个收尾</p>
        <h1>日报 / 周报</h1>
        <p class="page-subtitle">日报逐条记录，周报自动汇总本周日报。</p>
      </div>
      <FileText :size="25" class="reports-heading-icon" />
    </div>

    <div class="reports-tabs" role="tablist" aria-label="日报和周报">
      <button type="button" :class="{ active: tab === 'daily' }" @click="tab = 'daily'">日报</button>
      <button type="button" :class="{ active: tab === 'weekly' }" @click="tab = 'weekly'">周报</button>
      <button type="button" :class="{ active: tab === 'history' }" @click="tab = 'history'">历史记录</button>
    </div>

    <template v-if="tab === 'daily'">
      <form class="report-form" @submit.prevent="submit">
        <div class="report-field"><label for="report-content">新增一条日报 <b>*</b></label><textarea id="report-content" v-model="content" rows="2" maxlength="300" placeholder="例如：完成日报 / 周报页面的交互原型" /></div>
        <div class="report-form-row"><label class="report-field"><span>当前进度</span><input v-model.number="progress" type="number" min="0" max="100" step="1" /> </label><div class="report-progress-hint">0 — 100%</div></div>
        <div class="report-form-actions"><button class="report-secondary" type="button" @click="content = ''; progress = 0">清空</button><button class="report-primary" type="submit"><Plus :size="14" /> 添加日报</button></div>
      </form>

      <div class="reports-list-heading"><strong>今天的日报记录</strong><button class="report-copy" type="button" :disabled="!todayEntries.length" @click="copyToday"><ClipboardCopy :size="13" />复制全部日报</button></div>
      <div v-if="todayEntries.length" class="reports-list">
        <article v-for="entry in todayEntries" :key="entry.id" class="report-entry">
          <div class="report-entry-body"><p>{{ entry.content }}</p><small>{{ entry.task_id ? '来自今天的工作任务' : '手动填写' }}</small></div>
          <label class="report-entry-progress"><span>进度</span><input :value="entry.progress" type="number" min="0" max="100" @change="changeProgress(entry, ($event.target as HTMLInputElement).value)" />%</label>
        </article>
      </div>
      <div v-else class="reports-empty"><Sparkles :size="19" /><span>今天还没有日报，记录一条进展吧。</span></div>
    </template>

    <template v-else-if="tab === 'weekly'">
      <div class="weekly-card">
        <div class="weekly-card-heading"><div><span>本周周报</span><strong>{{ dateKey(weekStart) }} — {{ dateKey(weekEnd) }}</strong></div><button class="report-copy" type="button" :disabled="!weekEntries.length" @click="copyWeek"><ClipboardCopy :size="13" />复制周报</button></div>
        <div class="weekly-stats"><div><b>{{ weekEntries.length }}</b><span>条日报</span></div><div><b>{{ averageProgress }}%</b><span>平均进度</span></div><div><b>{{ new Set(weekEntries.map((entry) => entry.date)).size }}</b><span>记录天数</span></div></div>
        <div v-if="weekEntries.length" class="weekly-entries"><div v-for="entry in weekEntries" :key="entry.id" class="weekly-entry"><time>{{ displayDate(entry.date) }}</time><span>{{ entry.content }}</span><b>{{ entry.progress }}%</b></div></div>
        <div v-else class="reports-empty"><Sparkles :size="19" /><span>本周还没有日报，周报会自动汇总这里的记录。</span></div>
      </div>
    </template>

    <template v-else>
      <div v-if="historyGroups.length" class="report-history"><section v-for="[date, items] in historyGroups" :key="date"><h2>{{ displayDate(date) }} <small>{{ date }}</small></h2><div v-for="entry in items" :key="entry.id" class="history-entry"><span>{{ entry.content }}</span><b>{{ entry.progress }}%</b></div></section></div>
      <div v-else class="reports-empty"><Save :size="19" /><span>保存后的日报会出现在这里。</span></div>
    </template>
  </section>
</template>
