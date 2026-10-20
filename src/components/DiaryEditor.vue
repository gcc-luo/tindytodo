<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { EditorContent, VueNodeViewRenderer, useEditor } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import TextAlign from '@tiptap/extension-text-align'
import { TextStyleKit } from '@tiptap/extension-text-style'
import Link from '@tiptap/extension-link'
import { Trash2 } from 'lucide-vue-next'
import DiaryImageNode from './DiaryImageNode.vue'
import { DiaryImage } from '../lib/diaryImageExtension'
import { sanitizeDiaryHtml } from '../lib/diaryEditor'
import { importDiaryImage, isNativeApp, putDiaryAsset, resolveDiaryAsset } from '../api'
import type { DiaryEntry, DiaryTag } from '../types'

export interface DiarySnapshot {
  title: string
  content_html: string
  tag_ids: number[]
}

const props = defineProps<{
  entry: DiaryEntry
  tags: DiaryTag[]
  saveStatus?: 'idle' | 'saving' | 'saved' | 'error'
  saveError?: string
}>()

const emit = defineEmits<{
  save: [snapshot: DiarySnapshot]
  retry: []
  trash: []
}>()

const title = ref(props.entry.title)
const selectedTagIds = ref([...props.entry.tag_ids])
const titleInput = ref<HTMLInputElement>()
watch(() => props.entry.tag_ids, (ids) => { selectedTagIds.value = [...ids] })
const fileInput = ref<HTMLInputElement>()
const emojiMenuOpen = ref(false)
const imageError = ref('')
const fontFamily = ref('Microsoft YaHei')
const fontSize = ref('16px')
const activeTagIds = computed(() => selectedTagIds.value)
const statusText = computed(() => ({
  idle: '尚未保存', saving: '正在保存', saved: '已保存', error: '保存失败',
}[props.saveStatus ?? 'idle']))

const imageNode = DiaryImage.extend({
  addNodeView() {
    return VueNodeViewRenderer(DiaryImageNode)
  },
})

const editor = useEditor({
  content: sanitizeDiaryHtml(props.entry.content_html),
  extensions: [
    StarterKit.configure({
      link: false,
      underline: false,
      undoRedo: { depth: 100 },
    }),
    TextStyleKit,
    Link.configure({ openOnClick: false, autolink: true, protocols: ['mailto'] }),
    TextAlign.configure({ types: ['heading', 'paragraph'] }),
    imageNode,
  ],
  editorProps: {
    attributes: { class: 'diary-editor-prosemirror', spellcheck: 'true' },
    handlePaste: (_view, event) => {
      const items = [...(event.clipboardData?.items ?? [])]
      const image = items.find((item) => item.kind === 'file' && item.type.startsWith('image/'))
      const file = image?.getAsFile()
      if (!file) return false
      event.preventDefault()
      void insertImageFile(file)
      return true
    },
  },
  onUpdate: ({ editor: activeEditor }) => publishSnapshot(activeEditor.getHTML()),
})

function publishSnapshot(contentHtml = editor.value?.getHTML() ?? props.entry.content_html) {
  emit('save', {
    title: title.value,
    content_html: sanitizeDiaryHtml(contentHtml),
    tag_ids: [...selectedTagIds.value],
  })
}

function updateTitle(event: Event) {
  title.value = (event.target as HTMLInputElement).value
  publishSnapshot()
}

function toggleTag(id: number) {
  selectedTagIds.value = selectedTagIds.value.includes(id)
    ? selectedTagIds.value.filter((tagId) => tagId !== id)
    : [...selectedTagIds.value, id]
  publishSnapshot()
}

function insertEmoji(emoji: string) {
  editor.value?.chain().focus().insertContent(emoji).run()
  emojiMenuOpen.value = false
}

function setFont(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  fontFamily.value = value
  editor.value?.chain().focus().setFontFamily(value).run()
}

function setSize(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  fontSize.value = value
  editor.value?.chain().focus().setFontSize(value).run()
}

function setColor(event: Event) {
  editor.value?.chain().focus().setColor((event.target as HTMLInputElement).value).run()
}

function setAlignment(event: Event) {
  editor.value?.chain().focus().setTextAlign((event.target as HTMLSelectElement).value).run()
}

function setLink() {
  const href = window.prompt('输入链接地址')?.trim()
  if (!href) return
  if (!/^(https?:\/\/|mailto:|#)/i.test(href)) {
    imageError.value = '链接需以 http、https、mailto 或 # 开头'
    return
  }
  imageError.value = ''
  editor.value?.chain().focus().extendMarkRange('link').setLink({ href }).run()
}

async function insertImageFile(file: File) {
  imageError.value = ''
  if (!['image/png', 'image/jpeg', 'image/gif', 'image/webp'].includes(file.type)) {
    imageError.value = '图片格式不支持，请选择 PNG、JPEG、GIF 或 WebP'
    return
  }
  try {
    let id: string
    let source: string | null = null
    if (isNativeApp) {
      const imported = await importDiaryImage(props.entry.id)
      if (!imported) return
      id = imported.id
      source = imported.src
    } else {
      id = crypto.randomUUID()
      await putDiaryAsset({
        id,
        entryId: props.entry.id,
        mimeType: file.type,
        bytes: new Uint8Array(await file.arrayBuffer()),
      })
      source = await resolveDiaryAsset(id)
    }
    if (!source) throw new Error('图片保存后无法读取')
    editor.value?.chain().focus().insertContent({
      type: 'diaryImage',
      attrs: { assetId: id, alt: file.name || '日记图片' },
    }).run()
    if (source.startsWith('blob:')) URL.revokeObjectURL(source)
  } catch (error) {
    imageError.value = error instanceof Error ? error.message : '图片插入失败，请重试'
  }
}

async function chooseImage() {
  imageError.value = ''
  if (isNativeApp) {
    const input = new File([], 'image.png', { type: 'image/png' })
    await insertImageFile(input)
  } else {
    fileInput.value?.click()
  }
}

async function handleFileChange(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (file) await insertImageFile(file)
}

const emojis = ['🙂', '😊', '❤️', '✨', '🌿', '☕', '🎉', '🌤️']

onMounted(() => {
  if (!props.entry.title) titleInput.value?.focus()
})
</script>

<template>
  <section class="diary-editor-shell">
    <header class="diary-entry-heading">
      <div>
        <p class="diary-date-line">{{ entry.entry_date }}</p>
        <input ref="titleInput" class="diary-title-input" :value="entry.title" placeholder="给这篇日记起个标题" maxlength="200" @input="updateTitle">
        <p class="diary-entry-meta">创建于 {{ new Date(entry.created_at).toLocaleString() }}</p>
      </div>
      <div class="diary-entry-actions">
        <button class="diary-move-trash" type="button" @click="emit('trash')"><Trash2 :size="14" />移入回收站</button>
        <div class="diary-save-indicator" :class="`is-${saveStatus ?? 'idle'}`" role="status">
          <span>{{ statusText }}</span>
          <button v-if="saveStatus === 'error'" type="button" @click="emit('retry')">重试</button>
        </div>
      </div>
    </header>

    <div v-if="tags.length" class="diary-entry-tags" aria-label="日记标签">
      <button
        v-for="tag in tags"
        :key="tag.id"
        type="button"
        class="diary-tag-chip"
        :class="{ active: activeTagIds.includes(tag.id) }"
        :aria-pressed="activeTagIds.includes(tag.id)"
        @click="toggleTag(tag.id)"
      >{{ tag.name }}</button>
    </div>

    <div class="diary-editor-toolbar" role="toolbar" aria-label="日记格式工具栏">
      <select aria-label="字体" :value="fontFamily" @change="setFont">
        <option value="Microsoft YaHei">微软雅黑</option><option value="Arial">Arial</option><option value="Georgia">Georgia</option><option value="宋体">宋体</option><option value="楷体">楷体</option>
      </select>
      <select aria-label="字号" :value="fontSize" @change="setSize">
        <option value="12px">小</option><option value="16px">中</option><option value="20px">大</option><option value="28px">特大</option>
      </select>
      <span class="diary-toolbar-divider" />
      <button type="button" title="标题" aria-label="标题" :class="{ active: editor?.isActive('heading') }" @click="editor?.chain().focus().toggleHeading({ level: 2 }).run()">H</button>
      <button type="button" title="粗体" aria-label="粗体" :class="{ active: editor?.isActive('bold') }" @click="editor?.chain().focus().toggleBold().run()"><strong>B</strong></button>
      <button type="button" title="斜体" aria-label="斜体" :class="{ active: editor?.isActive('italic') }" @click="editor?.chain().focus().toggleItalic().run()"><em>I</em></button>
      <button type="button" title="下划线" aria-label="下划线" :class="{ active: editor?.isActive('underline') }" @click="editor?.chain().focus().toggleUnderline().run()"><u>U</u></button>
      <button type="button" title="删除线" aria-label="删除线" :class="{ active: editor?.isActive('strike') }" @click="editor?.chain().focus().toggleStrike().run()"><s>S</s></button>
      <label class="diary-color-control" title="文字颜色" aria-label="文字颜色">A<input type="color" aria-label="选择文字颜色" value="#3a493d" @input="setColor"></label>
      <span class="diary-toolbar-divider" />
      <button type="button" title="无序列表" aria-label="无序列表" :class="{ active: editor?.isActive('bulletList') }" @click="editor?.chain().focus().toggleBulletList().run()">• ≡</button>
      <button type="button" title="有序列表" aria-label="有序列表" :class="{ active: editor?.isActive('orderedList') }" @click="editor?.chain().focus().toggleOrderedList().run()">1.</button>
      <button type="button" title="引用" aria-label="引用" :class="{ active: editor?.isActive('blockquote') }" @click="editor?.chain().focus().toggleBlockquote().run()">❝</button>
      <select aria-label="段落对齐" value="left" @change="setAlignment"><option value="left">左对齐</option><option value="center">居中</option><option value="right">右对齐</option><option value="justify">两端对齐</option></select>
      <button type="button" title="插入链接" aria-label="插入链接" @click="setLink">↗</button>
      <div class="diary-emoji-picker">
        <button type="button" title="插入表情" aria-label="插入表情" :aria-expanded="emojiMenuOpen" @click="emojiMenuOpen = !emojiMenuOpen">☺</button>
        <div v-if="emojiMenuOpen" class="diary-emoji-menu"><button v-for="emoji in emojis" :key="emoji" type="button" @click="insertEmoji(emoji)">{{ emoji }}</button></div>
      </div>
      <button type="button" title="插入图片" aria-label="插入图片" @click="chooseImage">▧</button>
      <input ref="fileInput" class="diary-image-file-input" type="file" accept="image/png,image/jpeg,image/gif,image/webp" @change="handleFileChange">
      <span class="diary-toolbar-divider" />
      <button type="button" title="撤销" aria-label="撤销" :disabled="!editor?.can().undo()" @click="editor?.chain().focus().undo().run()">↶</button>
      <button type="button" title="重做" aria-label="重做" :disabled="!editor?.can().redo()" @click="editor?.chain().focus().redo().run()">↷</button>
      <button type="button" title="清除格式" aria-label="清除格式" @click="editor?.chain().focus().unsetAllMarks().clearNodes().run()">Tx</button>
    </div>

    <p v-if="saveError" class="diary-editor-error" role="alert">{{ saveError }}</p>
    <p v-if="imageError" class="diary-editor-error" role="alert">{{ imageError }}</p>
    <EditorContent :editor="editor" class="diary-editor-content" />
  </section>
</template>
