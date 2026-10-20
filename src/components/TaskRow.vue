<script setup lang="ts">
import { CalendarDays, Clock3, Copy, GripVertical, Star } from 'lucide-vue-next'
import type { TodoTask } from '../types'
import { localDateKey } from '../lib/taskFilters'

const props = defineProps<{
  task: TodoTask
  selected: boolean
  listName?: string
  dragging: boolean
  dropTarget: boolean
}>()
const today = localDateKey()

const emit = defineEmits<{
  toggle: [id: number]
  select: [id: number]
  dragStart: [id: number]
  dragOver: [id: number]
  drop: [id: number]
  dragEnd: []
  copy: []
}>()

function dateLabel(value: string): string {
  const [year, month, day] = value.split('-').map(Number)
  const current = new Date()
  const date = new Date(year, month - 1, day)
  const difference = Math.round((date.getTime() - new Date(current.getFullYear(), current.getMonth(), current.getDate()).getTime()) / 86400000)
  if (difference === 0) return '今天'
  if (difference === 1) return '明天'
  if (difference === -1) return '昨天'
  return `${month}月${day}日`
}

function startDrag(event: DragEvent): void {
  event.dataTransfer?.setData('text/plain', String(props.task.id))
  emit('dragStart', props.task.id)
}
</script>

<template>
  <article
    class="task-row"
    :class="{ selected, completed: task.status === 1, dragging, 'drop-target': dropTarget }"
    draggable="true"
    @click="emit('select', task.id)"
    @dragstart="startDrag"
    @dragover.prevent="emit('dragOver', task.id)"
    @drop.prevent.stop="emit('drop', task.id)"
    @dragend="emit('dragEnd')"
  >
    <button
      class="drag-handle icon-button"
      type="button"
      title="拖动排序"
      aria-label="拖动排序"
      @click.stop
    >
      <GripVertical :size="15" />
    </button>
    <button
      class="check-button"
      :class="{ checked: task.status === 1 }"
      type="button"
      :aria-label="task.status === 1 ? '标记为未完成' : '标记为完成'"
      @click.stop="emit('toggle', task.id)"
    >
      <svg v-if="task.status === 1" viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8 3 3 6-6" /></svg>
    </button>
    <div class="task-content">
      <div class="task-title-line">
        <span class="task-title">{{ task.title }}</span>
        <span v-if="task.priority === 1" class="priority-mark" title="高优先级"><Star :size="13" fill="currentColor" /></span>
      </div>
      <p v-if="task.note" class="task-note">{{ task.note }}</p>
      <div v-if="task.due_date || task.due_time || listName" class="task-meta">
        <span v-if="task.due_date" :class="{ overdue: task.status === 0 && task.due_date < today }"><CalendarDays :size="12" />{{ dateLabel(task.due_date) }}</span>
        <span v-if="task.due_time"><Clock3 :size="12" />{{ task.due_time }}</span>
        <span v-if="listName" class="meta-list">{{ listName }}</span>
      </div>
    </div>
    <button
      class="task-copy-button icon-button"
      type="button"
      title="复制任务"
      aria-label="复制任务"
      @click.stop="emit('copy')"
    >
      <Copy :size="14" />
    </button>
  </article>
</template>
