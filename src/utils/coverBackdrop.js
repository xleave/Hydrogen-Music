const MIN_BACKDROP_SIDE = 160
const MAX_BACKDROP_SIDE = 384
const CACHE_LIMIT = 8
const backdropCache = new Map()

function targetLongSide() {
  const viewportLongSide = Math.max(
    Number(globalThis.innerWidth) || 0,
    Number(globalThis.innerHeight) || 0,
    1280,
  )
  return Math.max(
    MIN_BACKDROP_SIDE,
    Math.min(MAX_BACKDROP_SIDE, Math.round(viewportLongSide / 8)),
  )
}

function loadImage(source) {
  return new Promise((resolve, reject) => {
    const image = new Image()
    image.decoding = 'async'
    image.onload = () => resolve(image)
    image.onerror = () => reject(new Error('failed to decode cover image'))
    image.src = source
  })
}

function remember(source, value) {
  if (backdropCache.has(source)) backdropCache.delete(source)
  backdropCache.set(source, value)
  while (backdropCache.size > CACHE_LIMIT) {
    backdropCache.delete(backdropCache.keys().next().value)
  }
}

function drawSoftFallback(context, image, width, height) {
  const tinyWidth = Math.max(24, Math.round(width / 6))
  const tinyHeight = Math.max(24, Math.round(height / 6))
  const tiny = document.createElement('canvas')
  tiny.width = tinyWidth
  tiny.height = tinyHeight
  const tinyContext = tiny.getContext('2d', { alpha: false })
  if (!tinyContext) {
    context.drawImage(image, 0, 0, width, height)
    return
  }
  tinyContext.imageSmoothingEnabled = true
  tinyContext.imageSmoothingQuality = 'high'
  tinyContext.drawImage(image, 0, 0, tinyWidth, tinyHeight)
  context.imageSmoothingEnabled = true
  context.imageSmoothingQuality = 'high'
  context.drawImage(tiny, 0, 0, tinyWidth, tinyHeight, 0, 0, width, height)
}

export async function createCoverBackdrop(source) {
  if (!source) return null
  const cached = backdropCache.get(source)
  if (cached) return cached

  const image = await loadImage(source)
  const naturalWidth = Math.max(1, image.naturalWidth || image.width || 1)
  const naturalHeight = Math.max(1, image.naturalHeight || image.height || 1)
  const longSide = targetLongSide()
  const scale = Math.min(1, longSide / Math.max(naturalWidth, naturalHeight))
  const width = Math.max(1, Math.round(naturalWidth * scale))
  const height = Math.max(1, Math.round(naturalHeight * scale))

  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d', { alpha: false })
  if (!context) return source

  context.fillStyle = '#d8e7e9'
  context.fillRect(0, 0, width, height)
  context.imageSmoothingEnabled = true
  context.imageSmoothingQuality = 'high'

  // At this resolution ~5px of pre-blur expands to roughly the old 50px
  // full-screen blur. The expensive filter therefore runs once on a few
  // tens of thousands of pixels instead of every frame on the whole window.
  if ('filter' in context) {
    const blurRadius = Math.max(3, Math.min(6, longSide / 48))
    const overscan = Math.ceil(blurRadius * 2.5)
    context.filter = `blur(${blurRadius}px)`
    context.drawImage(
      image,
      -overscan,
      -overscan,
      width + overscan * 2,
      height + overscan * 2,
    )
    context.filter = 'none'
  } else {
    drawSoftFallback(context, image, width, height)
  }

  const backdrop = canvas.toDataURL('image/jpeg', 0.72)
  remember(source, backdrop)
  return backdrop
}

export function clearCoverBackdropCache() {
  backdropCache.clear()
}
