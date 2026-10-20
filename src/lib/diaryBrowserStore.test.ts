import { afterEach, describe, expect, it, vi } from 'vitest'
import type { DiaryData } from '../types'
import {
  createDiaryEntry,
  createDiaryTag,
  getDiaryData,
  restoreDiaryEntry,
  saveDiaryEntry,
  trashDiaryEntry,
} from './diaryBrowserStore'

function installStorage() {
  const values = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
  })
}

afterEach(() => vi.unstubAllGlobals())

describe('diary browser storage', () => {
  it('persists entries, tags, content and trash/restore operations', async () => {
    installStorage()
    const tag = await createDiaryTag('生活记录')
    const entry = await createDiaryEntry('2026-10-20')
    await saveDiaryEntry(entry.id, '今天', '<p>河边散步</p>', [tag.id])

    let data: DiaryData = await getDiaryData()
    expect(data.entries[0]).toMatchObject({ title: '今天', content_html: '<p>河边散步</p>', tag_ids: [tag.id] })
    expect(data.tags).toEqual([tag])

    await trashDiaryEntry(entry.id)
    data = await getDiaryData()
    expect(data.entries[0].deleted_at).not.toBeNull()

    await restoreDiaryEntry(entry.id)
    data = await getDiaryData()
    expect(data.entries[0].deleted_at).toBeNull()
  })
})
