const ROOT_MARGIN_PX = 520

export function attachLyricPaintCulling(area) {
  if (!(area instanceof Element) || typeof IntersectionObserver === 'undefined') {
    return { refresh() {}, dispose() {} }
  }

  const observed = new Set()
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      // visibility keeps the original layout box intact, so lyric positions,
      // scroll inertia and transition geometry are unchanged. It only skips
      // painting/compositing lines well outside the lyric viewport.
      entry.target.style.visibility = entry.isIntersecting ? '' : 'hidden'
    }
  }, {
    root: area,
    rootMargin: `${ROOT_MARGIN_PX}px 0px`,
    threshold: 0,
  })

  function refresh() {
    for (const line of area.querySelectorAll('.lyric-line')) {
      if (observed.has(line)) continue
      observed.add(line)
      observer.observe(line)
    }
    for (const line of [...observed]) {
      if (line.isConnected && area.contains(line)) continue
      observer.unobserve(line)
      observed.delete(line)
    }
  }

  const mutationObserver = new MutationObserver(refresh)
  mutationObserver.observe(area, { childList: true, subtree: true })
  refresh()

  return {
    refresh,
    dispose() {
      mutationObserver.disconnect()
      observer.disconnect()
      for (const line of observed) line.style.visibility = ''
      observed.clear()
    },
  }
}
