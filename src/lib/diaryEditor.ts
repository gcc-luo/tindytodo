const ALLOWED_TAGS = new Set([
  'P', 'H1', 'H2', 'H3', 'H4', 'STRONG', 'B', 'EM', 'I', 'U', 'S', 'DEL',
  'UL', 'OL', 'LI', 'BLOCKQUOTE', 'PRE', 'CODE', 'BR', 'HR', 'SPAN', 'A', 'IMG',
])
const DROP_TAGS = new Set(['SCRIPT', 'STYLE', 'IFRAME', 'OBJECT', 'EMBED', 'SVG', 'MATH', 'LINK', 'META'])
const ASSET_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const COLORS = /^(#[0-9a-f]{3,8}|rgba?\([\d\s.,%]+\)|[a-z]{1,24})$/i
const FONTS = new Set([
  'Arial', 'Georgia', 'Tahoma', 'Verdana', 'Times New Roman', 'Courier New',
  'Microsoft YaHei', '微软雅黑', '宋体', '黑体', '楷体', 'sans-serif', 'serif', 'monospace',
])

export function textFromDiaryHtml(html: string): string {
  if (typeof DOMParser === 'undefined') {
    return html
      .replace(/<(script|style|iframe|object|embed)\b[^>]*>[\s\S]*?<\/\1\s*>/gi, ' ')
      .replace(/<!--[^]*?-->/g, ' ')
      .replace(/<\/?(?:br|p|div|li|h[1-6])\b[^>]*>/gi, ' ')
      .replace(/<[^>]*>/g, ' ')
      .replace(/&nbsp;/gi, ' ')
      .replace(/&amp;/gi, '&')
      .replace(/&lt;/gi, '<')
      .replace(/&gt;/gi, '>')
      .replace(/&quot;/gi, '"')
      .replace(/&#39;/gi, "'")
      .replace(/\s+/g, ' ')
      .trim()
  }
  const document = new DOMParser().parseFromString(html, 'text/html')
  document.querySelectorAll('script, style, iframe, object, embed').forEach((node) => node.remove())
  return (document.body.textContent ?? '').replace(/\s+/g, ' ').trim()
}

export function referencedDiaryAssetIds(html: string): string[] {
  const document = new DOMParser().parseFromString(html, 'text/html')
  const ids = new Set<string>()
  document.querySelectorAll('img[data-asset-id]').forEach((image) => {
    const id = image.getAttribute('data-asset-id')
    if (id && ASSET_ID.test(id)) ids.add(id)
  })
  return [...ids]
}

function cleanStyle(style: string): string {
  const values = new Map<string, string>()
  for (const declaration of style.split(';')) {
    const separator = declaration.indexOf(':')
    if (separator < 0) continue
    const property = declaration.slice(0, separator).trim().toLowerCase()
    const value = declaration.slice(separator + 1).trim().replace(/["']/g, '')
    if (/[<>]|url\s*\(|expression\s*\(/i.test(value)) continue
    if (property === 'text-align' && ['left', 'center', 'right', 'justify'].includes(value.toLowerCase())) {
      values.set(property, value.toLowerCase())
    } else if (property === 'font-size' && /^(8|9|1[0-9]|2[0-9]|3[0-6])(px|pt)$/.test(value)) {
      values.set(property, value)
    } else if (property === 'font-family') {
      const firstFont = value.split(',')[0].trim()
      if (FONTS.has(firstFont)) values.set(property, firstFont)
    } else if ((property === 'color' || property === 'background-color') && COLORS.test(value)) {
      values.set(property, value)
    }
  }
  return [...values].map(([property, value]) => `${property}: ${value}`).join('; ')
}

export function sanitizeDiaryHtml(html: string): string {
  const document = new DOMParser().parseFromString(html, 'text/html')
  document.querySelectorAll([...DROP_TAGS].join(',')).forEach((node) => node.remove())

  for (const element of [...document.body.querySelectorAll('*')]) {
    if (!element.isConnected) continue
    if (!ALLOWED_TAGS.has(element.tagName)) {
      element.replaceWith(...element.childNodes)
      continue
    }

    const tag = element.tagName
    const assetId = tag === 'IMG' ? element.getAttribute('data-asset-id') : null
    const href = tag === 'A' ? element.getAttribute('href') : null
    const title = element.getAttribute('title')
    const style = cleanStyle(element.getAttribute('style') ?? '')
    const alt = tag === 'IMG' ? element.getAttribute('alt') : null
    const width = tag === 'IMG' ? element.getAttribute('width') : null
    const height = tag === 'IMG' ? element.getAttribute('height') : null

    for (const attribute of [...element.attributes]) element.removeAttribute(attribute.name)
    if (style) element.setAttribute('style', style)
    if (title) element.setAttribute('title', title.slice(0, 200))

    if (tag === 'IMG') {
      if (!assetId || !ASSET_ID.test(assetId)) {
        element.remove()
        continue
      }
      element.setAttribute('data-asset-id', assetId)
      if (alt) element.setAttribute('alt', alt.slice(0, 300))
      if (width && /^\d{1,4}$/.test(width)) element.setAttribute('width', width)
      if (height && /^\d{1,4}$/.test(height)) element.setAttribute('height', height)
    } else if (tag === 'A' && href && /^(https?:|mailto:|#)/i.test(href.trim())) {
      element.setAttribute('href', href.trim())
      if (/^https?:/i.test(href)) element.setAttribute('rel', 'noopener noreferrer')
    }
  }

  return document.body.innerHTML
}
