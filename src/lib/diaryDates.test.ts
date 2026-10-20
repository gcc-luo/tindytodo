import { describe, expect, it } from 'vitest'
import { groupEntriesByDate, localDateKey, monthEntryCounts } from './diaryDates'
import type { DiaryEntry } from '../types'

const entry = (id: number, entry_date: string, created_at: string): DiaryEntry => ({
  id,
  entry_date,
  title: `条目 ${id}`,
  content_html: '<p>正文</p>',
  created_at,
  updated_at: created_at,
  deleted_at: null,
  tag_ids: [],
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
})
