import { describe, expect, it } from 'vitest'
import { diaryEntryPreview, filterDiaryEntries, groupEntriesByDate, localDateKey, monthCalendarCells, monthEntryCounts } from './diaryDates'
import type { DiaryEntry } from '../types'

const entry = (id: number, entry_date: string, created_at: string, title = `条目 ${id}`, content_html = '<p>正文</p>'): DiaryEntry => ({
  id, entry_date, title, content_html, created_at, updated_at: created_at, deleted_at: null, tag_ids: [],
})

describe('diary date navigation', () => {
  it('uses local calendar fields for diary date keys', () => {
    expect(localDateKey(new Date(2026, 9, 20, 23, 45))).toBe('2026-10-20')
  })

  it('groups active entries by date and orders newest first with stable ties', () => {
    const grouped = groupEntriesByDate([
      entry(1, '2026-10-20', '2026-10-20 09:00:00'),
      entry(2, '2026-10-20', '2026-10-20 15:00:00'),
      entry(3, '2026-10-20', '2026-10-20 15:00:00'),
      { ...entry(4, '2026-10-20', '2026-10-20 16:00:00'), deleted_at: '2026-10-20 16:01:00' },
    ])
    expect(grouped.get('2026-10-20')?.map((item) => item.id)).toEqual([3, 2, 1])
  })

  it('counts only active entries for the requested calendar month', () => {
    expect(monthEntryCounts([
      entry(1, '2026-10-20', '2026-10-20 09:00:00'),
      entry(2, '2026-10-20', '2026-10-20 15:00:00'),
      entry(3, '2026-11-01', '2026-11-01 08:00:00'),
      { ...entry(4, '2026-10-20', '2026-10-20 16:00:00'), deleted_at: '2026-10-20 16:01:00' },
    ], '2026-10')).toEqual(new Map([['2026-10-20', 2]]))
  })

  it('combines date, tag and title/body search and isolates trash entries', () => {
    const rows = [
      { ...entry(1, '2026-10-20', '2026-10-20 09:00:00', '河边', '<p>晨间散步</p>'), tag_ids: [7] },
      { ...entry(2, '2026-10-20', '2026-10-20 10:00:00', '跑步', '<p>公园</p>'), tag_ids: [8] },
      { ...entry(3, '2026-10-21', '2026-10-21 09:00:00', '河边', '<p>散步</p>'), tag_ids: [7] },
      { ...entry(4, '2026-10-20', '2026-10-20 11:00:00', '河边', '<p>晨间散步</p>'), tag_ids: [7], deleted_at: '2026-10-20 12:00:00' },
    ]
    expect(filterDiaryEntries(rows, { date: '2026-10-20', tagId: 7, query: '晨间散步', trash: false }).map((item) => item.id)).toEqual([1])
    expect(filterDiaryEntries(rows, { date: null, tagId: null, query: '晨间散步', trash: true }).map((item) => item.id)).toEqual([4])
  })

  it('builds a Sunday-first 42-day calendar grid with adjacent month dates', () => {
    const cells = monthCalendarCells(new Date(2026, 9, 20))
    expect(cells).toHaveLength(42)
    expect(cells[0].getDay()).toBe(0)
    expect(localDateKey(cells[0])).toBe('2026-09-27')
    expect(localDateKey(cells[41])).toBe('2026-11-07')
  })

  it('uses the first sentence for blank-title previews and truncates by Unicode characters', () => {
    const content = '今天去了河边散步，看到一只很可爱的小狗。后面还有一些内容。'
    expect(diaryEntryPreview(entry(1, '2026-10-20', 'now', '', `<p>${content}</p>`))).toBe('今天去了河边散步，看到一只很可爱的小狗。')
    expect([...diaryEntryPreview(entry(2, '2026-10-20', 'now', '', `<p>${'🌿'.repeat(60)}</p>`))]).toHaveLength(42)
  })
})
