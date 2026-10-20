import type { DiaryData, DiaryEntry, DiaryTag } from '../types'

const STORAGE_KEY = 'tinytodo-diary-v1'

const emptyData = (): DiaryData => ({ entries: [], tags: [] })

function readData(): DiaryData {
  const saved = localStorage.getItem(STORAGE_KEY)
  if (!saved) return emptyData()
  try {
    const value = JSON.parse(saved) as Partial<DiaryData>
    return {
      entries: Array.isArray(value.entries) ? value.entries : [],
      tags: Array.isArray(value.tags) ? value.tags : [],
    }
  } catch {
    return emptyData()
  }
}

function writeData(data: DiaryData): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
}

function nextId(items: Array<{ id: number }>): number {
  return Math.max(0, ...items.map((item) => item.id)) + 1
}

function timestamp(): string {
  return new Date().toISOString()
}

export async function getDiaryData(): Promise<DiaryData> {
  return readData()
}

export async function createDiaryEntry(entryDate: string): Promise<DiaryEntry> {
  const data = readData()
  const now = timestamp()
  const entry: DiaryEntry = {
    id: nextId(data.entries), entry_date: entryDate, title: '', content_html: '<p></p>',
    created_at: now, updated_at: now, deleted_at: null, tag_ids: [],
  }
  writeData({ ...data, entries: [...data.entries, entry] })
  return entry
}

export async function saveDiaryEntry(id: number, title: string, contentHtml: string, tagIds: number[]): Promise<DiaryEntry> {
  const data = readData()
  const index = data.entries.findIndex((entry) => entry.id === id)
  if (index < 0) throw new Error('找不到这篇日记，它可能已经被删除')
  const uniqueTagIds = [...new Set(tagIds)]
  if (uniqueTagIds.some((tagId) => !data.tags.some((tag) => tag.id === tagId))) {
    throw new Error('有标签已不存在，请刷新后重试')
  }
  const entry = { ...data.entries[index], title, content_html: contentHtml, updated_at: timestamp(), tag_ids: uniqueTagIds }
  const entries = [...data.entries]
  entries[index] = entry
  writeData({ ...data, entries })
  return entry
}

export async function createDiaryTag(name: string): Promise<DiaryTag> {
  const cleaned = name.trim()
  if (!cleaned) throw new Error('标签名称不能为空')
  if (cleaned.length > 40) throw new Error('标签名称不能超过 40 个字符')
  const data = readData()
  if (data.tags.some((tag) => tag.name.localeCompare(cleaned, undefined, { sensitivity: 'accent' }) === 0)) {
    throw new Error('已有同名标签，请换一个名称')
  }
  const tag = { id: nextId(data.tags), name: cleaned, created_at: timestamp() }
  writeData({ ...data, tags: [...data.tags, tag] })
  return tag
}

export async function renameDiaryTag(id: number, name: string): Promise<DiaryTag> {
  const cleaned = name.trim()
  if (!cleaned) throw new Error('标签名称不能为空')
  if (cleaned.length > 40) throw new Error('标签名称不能超过 40 个字符')
  const data = readData()
  if (data.tags.some((tag) => tag.id !== id && tag.name.toLocaleLowerCase() === cleaned.toLocaleLowerCase())) {
    throw new Error('已有同名标签，请换一个名称')
  }
  const index = data.tags.findIndex((tag) => tag.id === id)
  if (index < 0) throw new Error('找不到这个日记标签')
  const tag = { ...data.tags[index], name: cleaned }
  const tags = [...data.tags]
  tags[index] = tag
  writeData({ ...data, tags })
  return tag
}

export async function deleteDiaryTag(id: number): Promise<void> {
  const data = readData()
  writeData({
    entries: data.entries.map((entry) => ({ ...entry, tag_ids: entry.tag_ids.filter((tagId) => tagId !== id) })),
    tags: data.tags.filter((tag) => tag.id !== id),
  })
}

export async function trashDiaryEntry(id: number): Promise<void> {
  const data = readData()
  const now = timestamp()
  const entries = data.entries.map((entry) => entry.id === id ? { ...entry, deleted_at: now } : entry)
  if (!entries.some((entry) => entry.id === id)) throw new Error('找不到这篇日记')
  writeData({ ...data, entries })
}

export async function restoreDiaryEntry(id: number): Promise<void> {
  const data = readData()
  let found = false
  const entries = data.entries.map((entry) => {
    if (entry.id !== id) return entry
    found = true
    return { ...entry, deleted_at: null }
  })
  if (!found) throw new Error('找不到这篇日记')
  writeData({ ...data, entries })
}

export async function purgeDiaryEntry(id: number): Promise<void> {
  const data = readData()
  writeData({ ...data, entries: data.entries.filter((entry) => entry.id !== id) })
}
