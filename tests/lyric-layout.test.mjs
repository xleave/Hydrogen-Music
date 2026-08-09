import assert from 'node:assert/strict'
import test from 'node:test'

import {
  calculateLyricLineHeight,
  centeredLyricBlockOffset,
  centeredLyricLineOffset,
  lyricBlurRadius,
} from '../src/utils/lyricLayout.mjs'

test('the active timed lyric stays at the vertical center of the lyric viewport', () => {
  const viewportCenter = 400
  const measuredLines = [
    { top: 0, height: 58 },
    { top: 68, height: 92 },
    { top: 170, height: 64 },
  ]

  for (const line of measuredLines) {
    const trackOffset = centeredLyricLineOffset(line.top, line.height)
    const renderedLineCenter = viewportCenter + trackOffset + line.top + line.height / 2

    assert.equal(renderedLineCenter, viewportCenter)
  }
})

test('untimed lyrics center the complete text block instead of pretending a line is active', () => {
  const totalHeight = 360
  const trackOffset = centeredLyricBlockOffset(totalHeight)

  assert.equal(trackOffset + totalHeight / 2, 0)
})

test('lyric blur is symmetric around the active line and disabled without a timeline', () => {
  assert.equal(lyricBlurRadius(3, 5, true, true), lyricBlurRadius(7, 5, true, true))
  assert.equal(lyricBlurRadius(5, 5, true, true), 0)
  assert.equal(lyricBlurRadius(7, -1, true, true), 0)
  assert.equal(lyricBlurRadius(7, 5, true, false), 0)
  assert.equal(lyricBlurRadius(7, 5, false, true), 0)
})

test('line height includes only lyric variants selected and available for this track', () => {
  const height = calculateLyricLineHeight(
    { original: 20, trans: 13, roma: 12 },
    ['original', 'trans', 'roma'],
    { original: true, trans: true, roma: false },
  )

  assert.equal(height, (20 + 13) * 1.5 + 30)
})
