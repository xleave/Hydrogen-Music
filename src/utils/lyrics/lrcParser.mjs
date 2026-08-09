const TIMESTAMP_PATTERN = /\[(\d{1,3}):([0-5]\d)(?:[.:](\d{1,3}))?\]/g
const OFFSET_PATTERN = /^\[offset:([+-]?\d+)\]\s*$/i
const METADATA_PATTERN = /^\[(?:ar|al|ti|au|by|re|ve|length|offset):.*\]\s*$/i
const MATCH_TOLERANCE_MS = 50
const INLINE_TRANSLATION_SEPARATOR = '\u2009'

function emptyResult() {
  return {
    lines: [],
    synchronized: false,
    availability: { original: false, trans: false, roma: false },
  }
}

function timestampMilliseconds(match, offsetMilliseconds) {
  const fraction = (match[3] || '').padEnd(3, '0')
  const milliseconds = Number(match[1]) * 60_000
    + Number(match[2]) * 1_000
    + Number(fraction || 0)
    + offsetMilliseconds
  return Math.max(0, milliseconds)
}

function lyricText(value, extractInlineTranslation) {
  const text = value.trim()
  if (!extractInlineTranslation) return { text }

  const separatorIndex = text.indexOf(INLINE_TRANSLATION_SEPARATOR)
  if (separatorIndex === -1) return { text }

  const lyric = text.slice(0, separatorIndex).trim()
  const inlineTranslation = text.slice(separatorIndex + INLINE_TRANSLATION_SEPARATOR.length).trim()
  if (!lyric || !inlineTranslation) return { text: lyric || inlineTranslation }
  return { text: lyric, inlineTranslation }
}

function parseDocument(text, extractInlineTranslation = false) {
  const sourceLines = String(text || '').split(/\r?\n/)
  let offsetMilliseconds = 0
  for (const sourceLine of sourceLines) {
    const offset = sourceLine.trim().match(OFFSET_PATTERN)
    if (offset) offsetMilliseconds = Number(offset[1])
  }

  const timed = []
  const plain = []
  let order = 0
  for (const sourceLine of sourceLines) {
    const matches = [...sourceLine.matchAll(TIMESTAMP_PATTERN)]
    if (matches.length) {
      const parsedText = lyricText(sourceLine.replace(TIMESTAMP_PATTERN, ''), extractInlineTranslation)
      if (!parsedText.text) continue
      for (const match of matches) {
        timed.push({
          milliseconds: timestampMilliseconds(match, offsetMilliseconds),
          order: order++,
          ...parsedText,
        })
      }
      continue
    }

    const text = sourceLine.trim()
    if (text && !METADATA_PATTERN.test(text)) plain.push(lyricText(text, extractInlineTranslation))
  }
  timed.sort((left, right) => left.milliseconds - right.milliseconds || left.order - right.order)
  return { timed, plain }
}

function nearestText(entries, milliseconds) {
  let low = 0
  let high = entries.length
  while (low < high) {
    const middle = Math.floor((low + high) / 2)
    if (entries[middle].milliseconds < milliseconds) low = middle + 1
    else high = middle
  }

  let nearest = null
  for (const index of [low - 1, low]) {
    const entry = entries[index]
    if (!entry) continue
    const distance = Math.abs(entry.milliseconds - milliseconds)
    if (distance > MATCH_TOLERANCE_MS) continue
    if (!nearest || distance < nearest.distance) nearest = { distance, text: entry.text }
  }
  return nearest?.text
}

function timedLyrics(original, translated, romanized) {
  return original.timed.map((entry) => ({
    lyric: entry.text,
    time: entry.milliseconds / 1000,
    tlyric: nearestText(translated.timed, entry.milliseconds) ?? entry.inlineTranslation,
    rlyric: nearestText(romanized.timed, entry.milliseconds),
  }))
}

function plainLyrics(original, translated, romanized) {
  return original.plain.map((entry, index) => ({
    active: true,
    lyric: entry.text,
    time: null,
    tlyric: translated.plain[index]?.text ?? entry.inlineTranslation,
    rlyric: romanized.plain[index]?.text,
  }))
}

export function parseLyrics(value) {
  if (!value?.lrc?.lyric) return emptyResult()

  const original = parseDocument(value.lrc.lyric, true)
  const translated = parseDocument(value.tlyric?.lyric)
  const romanized = parseDocument(value.romalrc?.lyric)
  const synchronized = original.timed.length > 0
  let lines = synchronized
    ? timedLyrics(original, translated, romanized)
    : plainLyrics(original, translated, romanized)

  if (lines.some((line) => line.lyric.includes('纯音乐'))) {
    lines = [{ lyric: '纯音乐，请欣赏', time: 0 }]
  }

  return {
    lines,
    synchronized,
    availability: {
      original: lines.some((line) => Boolean(line.lyric)),
      trans: lines.some((line) => Boolean(line.tlyric)),
      roma: lines.some((line) => Boolean(line.rlyric)),
    },
  }
}
