// @vitest-environment jsdom
import 'fake-indexeddb/auto'
import { afterEach, describe, expect, it } from 'vitest'
import { deleteDiaryAsset, getDiaryAsset, putDiaryAsset } from './diaryBrowserStore'

const bytes = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10])

afterEach(async () => {
  const databases = await indexedDB.databases()
  await Promise.all(databases.map(({ name }) => name && new Promise<void>((resolve, reject) => {
    const request = indexedDB.deleteDatabase(name)
    request.onsuccess = () => resolve()
    request.onerror = () => reject(request.error)
    request.onblocked = () => resolve()
  })))
})

describe('diary browser assets', () => {
  it('stores, reads and deletes image bytes', async () => {
    await putDiaryAsset({ id: 'asset-1', entryId: 3, mimeType: 'image/png', bytes })
    expect(await getDiaryAsset('asset-1')).toEqual(bytes)
    await deleteDiaryAsset('asset-1')
    expect(await getDiaryAsset('asset-1')).toBeNull()
  })
})
