const TIMESTAMP_PATTERN = /\[(\d{1,3}):([0-5]\d)(?:[.:](\d{1,3}))?\]/g
const OFFSET_PATTERN = /^\[offset:([+-]?\d+)\]\s*$/i
const METADATA_PATTERN = /^\[(?:ar|al|ti|au|by|re|ve|length|offset):.*\]\s*$/i
const MATCH_TOLERANCE_MS = 50

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

function parseDocument(text) {
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
      const text = sourceLine.replace(TIMESTAMP_PATTERN, '').trim()
      if (!text) continue
      for (const match of matches) {
        timed.push({
          milliseconds: timestampMilliseconds(match, offsetMilliseconds),
          order: order++,
          text,
        })
      }
      continue
    }

    const text = sourceLine.trim()
    if (text && !METADATA_PATTERN.test(text)) plain.push(text)
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
    tlyric: nearestText(translated.timed, entry.milliseconds),
    rlyric: nearestText(romanized.timed, entry.milliseconds),
  }))
}

function plainLyrics(original, translated, romanized) {
  return original.plain.map((text, index) => ({
    active: true,
    lyric: text,
    time: null,
    tlyric: translated.plain[index],
    rlyric: romanized.plain[index],
  }))
}

export function parseLyrics(value) {
  if (!value?.lrc?.lyric) return emptyResult()

  const original = parseDocument(value.lrc.lyric)
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
