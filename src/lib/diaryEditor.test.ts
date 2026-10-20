// @vitest-environment jsdom
import { describe, expect, it } from 'vitest'
import { referencedDiaryAssetIds, sanitizeDiaryHtml, textFromDiaryHtml } from './diaryEditor'

const first = '01234567-89ab-cdef-0123-456789abcdef'
const second = '11234567-89ab-cdef-0123-456789abcdef'

describe('diary editor HTML helpers', () => {
  it('extracts searchable text without script or style contents', () => {
    expect(textFromDiaryHtml('<p>今天 <strong>散步</strong></p><script>alert(1)</script><style>.x{}</style>')).toBe('今天 散步')
  })

  it('deduplicates only supported image asset references', () => {
    expect(referencedDiaryAssetIds(`<img data-asset-id="${first}"><img data-asset-id="${first}"><img data-asset-id="${second}"><img data-asset-id="../bad">`)).toEqual([first, second])
  })

  it('removes scripts, event handlers, external images and unsafe links', () => {
    const html = sanitizeDiaryHtml(`<p onclick="alert(1)">安全 <strong>文字</strong><script>alert(1)</script><a href="javascript:alert(1)">链接</a><img src="https://evil.test/a.png"><img data-asset-id="${first}" src="https://evil.test/a.png"></p>`)
    expect(html).not.toContain('script')
    expect(html).not.toContain('onclick')
    expect(html).not.toContain('javascript:')
    expect(html).not.toContain('evil.test')
    expect(html).toContain(`data-asset-id="${first}"`)
  })
})
