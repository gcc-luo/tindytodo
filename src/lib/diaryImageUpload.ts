import type { DiaryAssetBlob } from './diaryBrowserStore'

export async function persistDiaryImageFile(
  file: File,
  entryId: number,
  saveAsset: (asset: DiaryAssetBlob) => Promise<void>,
  createId: () => string = () => crypto.randomUUID(),
): Promise<string> {
  const id = createId()
  await saveAsset({
    id,
    entryId,
    mimeType: file.type,
    bytes: new Uint8Array(await file.arrayBuffer()),
  })
  return id
}
