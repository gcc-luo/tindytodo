import type { DiaryEntry } from '../types'
import { textFromDiaryHtml } from './diaryEditor'

export function localDateKey(date = new Date()): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function monthCalendarCells(date: Date): Date[] {
  const first = new Date(date.getFullYear(), date.getMonth(), 1)
  const firstCell = new Date(first.getFullYear(), first.getMonth(), 1 - first.getDay())
  return Array.from({ length: 42 }, (_, index) => {
    const cell = new Date(firstCell)
    cell.setDate(firstCell.getDate() + index)
    return cell
  })
}

export function diaryEntryPreview(entry: DiaryEntry, maxCharacters = 42): string {
  const title = entry.title.trim()
  if (title) return Array.from(title).slice(0, maxCharacters).join('')
  const text = textFromDiaryHtml(entry.content_html)
  if (!text) return '（无标题）'
  const punctuation = text.search(/[。！？.!?]/u)
  const firstSentence = punctuation >= 0 ? text.slice(0, punctuation + 1) : text
  return Array.from(firstSentence).slice(0, maxCharacters).join('')
}

export function groupEntriesByDate(entries: DiaryEntry[]): Map<string, DiaryEntry[]> {
  const grouped = new Map<string, DiaryEntry[]>()
  const activeEntries = entries
    .filter((entry) => entry.deleted_at === null)
    .slice()
    .sort((a, b) => b.created_at.localeCompare(a.created_at) || b.id - a.id)

  for (const entry of activeEntries) {
    const sameDay = grouped.get(entry.entry_date) ?? []
    sameDay.push(entry)
    grouped.set(entry.entry_date, sameDay)
  }

  return grouped
}

export function monthEntryCounts(entries: DiaryEntry[], month: string): Map<string, number> {
  const counts = new Map<string, number>()
  for (const entry of entries) {
    if (entry.deleted_at !== null || !entry.entry_date.startsWith(`${month}-`)) continue
    counts.set(entry.entry_date, (counts.get(entry.entry_date) ?? 0) + 1)
  }
  return counts
}

export interface DiaryEntryFilter {
  date: string | null
  tagId: number | null
  query: string
  trash: boolean
}

export function filterDiaryEntries(entries: DiaryEntry[], filter: DiaryEntryFilter): DiaryEntry[] {
  const query = filter.query.trim().toLocaleLowerCase()
  return entries.filter((entry) => {
    if (filter.trash ? entry.deleted_at === null : entry.deleted_at !== null) return false
    if (filter.date && entry.entry_date !== filter.date) return false
    if (filter.tagId !== null && !entry.tag_ids.includes(filter.tagId)) return false
    if (query && !`${entry.title} ${textFromDiaryHtml(entry.content_html)}`.toLocaleLowerCase().includes(query)) return false
    return true
  })
}
