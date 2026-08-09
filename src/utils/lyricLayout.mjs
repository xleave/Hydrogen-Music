export function calculateLyricLineHeight(sizes, preferences, availability) {
  const visibleSize = [
    preferences.includes('original') && availability.original ? Number(sizes.original) : 0,
    preferences.includes('trans') && availability.trans ? Number(sizes.trans) : 0,
    preferences.includes('roma') && availability.roma ? Number(sizes.roma) : 0,
  ].reduce((sum, value) => sum + (Number.isFinite(value) ? value : 0), 0)

  return visibleSize * 1.5 + 30
}

export function centeredLyricLineOffset(lineTop, lineHeight) {
  return -(Math.max(0, lineTop) + Math.max(0, lineHeight) / 2)
}

export function centeredLyricBlockOffset(totalHeight) {
  return -Math.max(0, totalHeight) / 2
}

export function lyricBlurRadius(index, activeIndex, enabled, synchronized) {
  if (!enabled || !synchronized || activeIndex < 0) return 0
  const distance = Math.abs(index - activeIndex)
  return distance > 0 ? Math.min(distance * 0.25, 1.8) : 0
}
