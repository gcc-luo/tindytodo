<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { CalendarDays, CornerDownLeft, X } from 'lucide-vue-next'
import { listen } from '@tauri-apps/api/event'
import { createTask, getShortcutInfo, hideQuickAdd } from '../api'
import { localDateKey } from '../lib/taskFilters'

const title = ref('')
const dueDate = ref('')
const input = ref<HTMLInputElement | null>(null)
const error = ref('')
const shortcutLabel = ref('Ctrl + Alt + Space')
let unlisten: (() => void) | undefined

async function focusInput(): Promise<void> {
  await nextTick()
  input.value?.focus()
}

async function save(): Promise<void> {
  if (!title.value.trim()) {
    error.value = '先写下要做的事'
    input.value?.focus()
    return
  }
  try {
    await createTask({ title: title.value.trim(), due_date: dueDate.value || null })
    title.value = ''
    dueDate.value = ''
    error.value = ''
    await hideQuickAdd()
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason)
  }
}

async function close(): Promise<void> {
  await hideQuickAdd()
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault()
    void close()
  }
  if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
    event.preventDefault()
    void save()
  }
}

onMounted(async () => {
  await focusInput()
  try {
    shortcutLabel.value = (await getShortcutInfo()).label
  } catch {
    // The quick window remains usable if shortcut status is unavailable.
  }
  try {
    unlisten = await listen('quick-add:show', () => {
      title.value = ''
      error.value = ''
      void focusInput()
    })
  } catch {
    // Browser preview runs without the Tauri event bridge.
  }
})
onBeforeUnmount(() => unlisten?.())
</script>

<template>
  <main class="quick-window" @keydown="onKeydown">
    <header class="quick-window-header">
      <div class="brand-mark small"><span></span></div>
      <div><strong>现在要记什么？</strong><small>按 {{ shortcutLabel }} 随时打开</small></div>
      <button class="icon-button" type="button" aria-label="关闭" @click="close"><X :size="17" /></button>
    </header>
    <form class="quick-window-form" @submit.prevent="save">
      <input ref="input" v-model="title" maxlength="300" placeholder="写下一件要做的事…" autocomplete="off" />
      <div class="quick-window-options">
        <label class="quick-date"><CalendarDays :size="14" /><input v-model="dueDate" type="date" :min="localDateKey()" /></label>
        <span v-if="error" class="quick-error">{{ error }}</span>
        <button class="quick-save" type="submit"><CornerDownLeft :size="15" /> 创建</button>
      </div>
    </form>
  </main>
</template>
