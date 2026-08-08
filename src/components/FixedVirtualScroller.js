import {
  computed,
  defineComponent,
  h,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from 'vue'

const OVERSCAN_ROWS = 4

/**
 * Fixed-height virtual list optimized for WebKitGTK.
 *
 * The native scroll container owns pixel scrolling. Vue only receives an
 * update when scrolling crosses an item boundary (or the viewport/items
 * change), so wheel/touchpad events do not cause a component update per pixel.
 * The public props/slot shape intentionally mirrors the small subset of
 * vue-virtual-scroller RecycleScroller used by Hydrogen Music.
 */
export const RecycleScroller = defineComponent({
  name: 'FixedVirtualScroller',
  inheritAttrs: false,
  props: {
    items: { type: Array, default: () => [] },
    itemSize: { type: Number, required: true },
    keyField: { type: String, default: 'id' },
  },
  setup(props, { attrs, slots, expose }) {
    const viewport = ref(null)
    const viewportHeight = ref(0)
    const firstVisible = ref(0)
    let resizeObserver = null

    const safeItemSize = computed(() => Math.max(1, Number(props.itemSize) || 1))
    const startIndex = computed(() => Math.max(0, firstVisible.value - OVERSCAN_ROWS))
    const visibleRows = computed(() => Math.max(1, Math.ceil(viewportHeight.value / safeItemSize.value)))
    const endIndex = computed(() => Math.min(
      props.items.length,
      startIndex.value + visibleRows.value + OVERSCAN_ROWS * 2,
    ))
    const visibleItems = computed(() => props.items.slice(startIndex.value, endIndex.value))
    const offsetY = computed(() => startIndex.value * safeItemSize.value)
    const contentHeight = computed(() => props.items.length * safeItemSize.value)

    function measure() {
      viewportHeight.value = viewport.value?.clientHeight || 0
    }

    function syncScrollIndex() {
      const element = viewport.value
      if (!element) return
      const next = Math.max(0, Math.floor(element.scrollTop / safeItemSize.value))
      if (next !== firstVisible.value) firstVisible.value = next
    }

    function onScroll() {
      syncScrollIndex()
    }

    function scrollToTop(behavior = 'auto') {
      viewport.value?.scrollTo({ top: 0, behavior })
      firstVisible.value = 0
    }

    function scrollToItem(index, behavior = 'auto') {
      const normalized = Math.max(0, Math.min(Number(index) || 0, Math.max(0, props.items.length - 1)))
      viewport.value?.scrollTo({ top: normalized * safeItemSize.value, behavior })
      syncScrollIndex()
    }

    expose({ scrollToTop, scrollToItem })

    onMounted(() => {
      measure()
      syncScrollIndex()
      if (typeof ResizeObserver !== 'undefined' && viewport.value) {
        resizeObserver = new ResizeObserver(measure)
        resizeObserver.observe(viewport.value)
      }
    })

    onBeforeUnmount(() => resizeObserver?.disconnect())

    watch(() => props.items.length, async () => {
      await nextTick()
      const element = viewport.value
      if (!element) return
      const maxScroll = Math.max(0, contentHeight.value - element.clientHeight)
      if (element.scrollTop > maxScroll) element.scrollTop = maxScroll
      syncScrollIndex()
    })

    return () => {
      const rootClass = ['fixed-virtual-list', attrs.class]
      const rootStyle = [attrs.style, {
        overflow: 'auto',
        position: 'relative',
        display: 'block',
      }]

      const rows = visibleItems.value.map((item, relativeIndex) => {
        const index = startIndex.value + relativeIndex
        const key = item?.[props.keyField] ?? index
        return h('div', {
          class: 'vue-recycle-scroller__item-view fixed-virtual-list-row',
          key,
          style: {
            width: '100%',
            height: `${safeItemSize.value}px`,
            boxSizing: 'border-box',
          },
        }, slots.default?.({ item, index }) || [])
      })

      const spacer = h('div', {
        class: 'fixed-virtual-list-spacer',
        style: {
          width: '100%',
          height: `${contentHeight.value}px`,
          position: 'relative',
          overflow: 'hidden',
        },
      }, [
        h('div', {
          class: 'fixed-virtual-list-window',
          style: {
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            transform: `translate3d(0, ${offsetY.value}px, 0)`,
          },
        }, rows),
      ])

      return h('div', {
        ...attrs,
        ref: viewport,
        class: rootClass,
        style: rootStyle,
        onScroll,
      }, [spacer, slots.after?.()])
    }
  },
})
