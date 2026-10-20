import { mergeAttributes, Node } from '@tiptap/core'

const assetIdPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

export const DiaryImage = Node.create({
  name: 'diaryImage',
  group: 'block',
  atom: true,
  selectable: true,
  draggable: false,

  addAttributes() {
    return {
      assetId: {
        default: null,
        parseHTML: (element) => element.getAttribute('data-asset-id'),
        renderHTML: (attributes) => assetIdPattern.test(attributes.assetId ?? '')
          ? { 'data-asset-id': attributes.assetId }
          : {},
      },
      alt: {
        default: '',
        parseHTML: (element) => element.getAttribute('alt') ?? '',
        renderHTML: (attributes) => attributes.alt ? { alt: attributes.alt } : {},
      },
    }
  },

  parseHTML() {
    return [{ tag: 'img[data-asset-id]' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['img', mergeAttributes(HTMLAttributes)]
  },
})
