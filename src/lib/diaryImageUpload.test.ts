import { describe, expect, it, vi } from 'vitest'
import { persistDiaryImageFile } from './diaryImageUpload'

describe('persistDiaryImageFile', () => {
  it('persists the supplied clipboard file bytes instead of choosing a path', async () => {
    const saveAsset = vi.fn(async () => undefined)
    const file = new File([new Uint8Array([137, 80, 78, 71])], 'paste.png', { type: 'image/png' })

    const id = await persistDiaryImageFile(file, 17, saveAsset, () => 'asset-id')

    expect(id).toBe('asset-id')
    expect(saveAsset).toHaveBeenCalledWith({
      id: 'asset-id',
      entryId: 17,
      mimeType: 'image/png',
      bytes: new Uint8Array([137, 80, 78, 71]),
    })
  })
})
