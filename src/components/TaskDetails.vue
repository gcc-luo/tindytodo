<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { CalendarDays, Check, ChevronDown, Clock3, Flag, ListChecks, MessageSquareText, Trash2, X } from 'lucide-vue-next'
import type { TodoList, TodoTask, TaskChanges } from '../types'

const props = defineProps<{ task: TodoTask; lists: TodoList[] }>()
const emit = defineEmits<{
  close: []
  update: [changes: TaskChanges]
  delete: [id: number]
}>()

const title = ref(props.task.title)
const note = ref(props.task.note ?? '')
const dueDate = ref(props.task.due_date ?? '')
const dueTime = ref(props.task.due_time ?? '')
const listId = ref(props.task.list_id?.toString() ?? '')
const priority = ref(props.task.priority)
const confirmDelete = ref(false)
const noteLength = computed(() => note.value.length)

watch(() => props.task.id, () => {
  title.value = props.task.title
  note.value = props.task.note ?? ''
  dueDate.value = props.task.due_date ?? ''
  dueTime.value = props.task.due_time ?? ''
  listId.value = props.task.list_id?.toString() ?? ''
  priority.value = props.task.priority
  confirmDelete.value = false
})

function saveTitle(): void {
  const trimmed = title.value.trim()
  if (!trimmed) {
    title.value = props.task.title
    return
  }
  if (trimmed !== props.task.title) emit('update', { title: trimmed })
}

function saveNote(): void {
  const value = note.value.trim() || null
  if (value !== props.task.note) emit('update', { note: value })
}

function saveDueDate(): void {
  if (!dueDate.value) dueTime.value = ''
  emit('update', { due_date: dueDate.value || null, due_time: dueDate.value ? (dueTime.value || null) : null })
}

function saveDueTime(): void {
  emit('update', { due_time: dueTime.value || null })
}
</script>

<template>
  <aside class="detail-panel" aria-label="任务详情">
    <div class="detail-header">
      <span class="detail-kicker">任务详情</span>
      <button class="icon-button" type="button" aria-label="关闭详情" @click="emit('close')"><X :size="17" /></button>
    </div>

    <textarea v-model="title" class="detail-title" rows="2" maxlength="300" aria-label="任务标题" @blur="saveTitle" />

    <div class="detail-fields">
      <label class="detail-field">
        <span><CalendarDays :size="15" /> 日期</span>
        <input v-model="dueDate" type="date" @change="saveDueDate" />
      </label>
      <label class="detail-field" :class="{ disabled: !dueDate }">
        <span><Clock3 :size="15" /> 时间</span>
        <input v-model="dueTime" type="time" :disabled="!dueDate" @change="saveDueTime" />
      </label>
      <label class="detail-field">
        <span><Flag :size="15" /> 优先级</span>
        <select v-model.number="priority" @change="emit('update', { priority })">
          <option :value="0">普通</option>
          <option :value="1">高优先级</option>
        </select>
        <ChevronDown class="select-chevron" :size="14" />
      </label>
      <label class="detail-field">
        <span><ListChecks :size="15" /> 清单</span>
        <select v-model="listId" @change="emit('update', { list_id: listId ? Number(listId) : null })">
          <option value="">无清单</option>
          <option v-for="list in lists" :key="list.id" :value="String(list.id)">{{ list.name }}</option>
        </select>
        <ChevronDown class="select-chevron" :size="14" />
      </label>
    </div>

    <div class="note-section">
      <div class="note-label"><span><MessageSquareText :size="15" /> 备注</span><span>{{ noteLength }}</span></div>
      <textarea v-model="note" class="note-input" placeholder="补充一些细节…" maxlength="2000" @blur="saveNote" />
    </div>

    <div class="detail-footer">
      <template v-if="confirmDelete">
        <span>确定删除这个任务？</span>
        <button class="text-button danger" type="button" @click="emit('delete', task.id)">删除</button>
        <button class="text-button" type="button" @click="confirmDelete = false">取消</button>
      </template>
      <button v-else class="delete-button" type="button" @click="confirmDelete = true"><Trash2 :size="14" /> 删除任务</button>
      <span class="autosave"><Check :size="11" /> 修改自动保存</span>
    </div>
  </aside>
</template>
