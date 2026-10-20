<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'
import { resolveDiaryAsset } from '../api'

const props = defineProps(nodeViewProps)
const source = ref('')
const failed = ref(false)
let currentObjectUrl = ''
let requestSequence = 0

watch(() => props.node.attrs.assetId as string | null, async (assetId) => {
  const sequence = ++requestSequence
  if (currentObjectUrl.startsWith('blob:')) URL.revokeObjectURL(currentObjectUrl)
  currentObjectUrl = ''
  source.value = ''
  failed.value = false
  if (!assetId) {
    failed.value = true
    return
  }
  try {
    const url = await resolveDiaryAsset(assetId)
    if (sequence !== requestSequence) {
      if (url?.startsWith('blob:')) URL.revokeObjectURL(url)
      return
    }
    if (!url) throw new Error('找不到图片文件')
    currentObjectUrl = url
    source.value = url
  } catch {
    if (sequence === requestSequence) failed.value = true
  }
}, { immediate: true })

onBeforeUnmount(() => {
  requestSequence += 1
  if (currentObjectUrl.startsWith('blob:')) URL.revokeObjectURL(currentObjectUrl)
})
</script>

<template>
  <NodeViewWrapper as="figure" class="diary-image-node" contenteditable="false">
    <img v-if="source" :src="source" :alt="String(props.node.attrs.alt || '日记图片')">
    <span v-else-if="failed" class="diary-image-error">图片暂时无法加载</span>
    <span v-else class="diary-image-loading">图片载入中…</span>
  </NodeViewWrapper>
</template>
