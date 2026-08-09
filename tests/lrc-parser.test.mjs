import assert from 'node:assert/strict'
import test from 'node:test'
import { parseLyrics } from '../src/utils/lyrics/lrcParser.mjs'

test('LRC offset applies to every timestamp on a line', () => {
  const parsed = parseLyrics({
    lrc: { lyric: '[offset:+100]\n[01:10.00][02:20.00]副歌' },
  })

  assert.equal(parsed.synchronized, true)
  assert.deepEqual(parsed.lines.map((line) => [line.time, line.lyric]), [
    [70.1, '副歌'],
    [140.1, '副歌'],
  ])
})

test('translation and romanization align within normal timestamp drift', () => {
  const parsed = parseLyrics({
    lrc: { lyric: '[00:01.000]Original' },
    tlyric: { lyric: '[00:01.008]Translation' },
    romalrc: { lyric: '[00:00.970]Romanization' },
  })

  assert.deepEqual(parsed.lines, [{
    lyric: 'Original',
    time: 1,
    tlyric: 'Translation',
    rlyric: 'Romanization',
  }])
  assert.deepEqual(parsed.availability, { original: true, trans: true, roma: true })
})

test('untimed lyrics remain untimed and ignore metadata tags', () => {
  const parsed = parseLyrics({
    lrc: { lyric: '[ar:Artist]\nFirst line\nSecond line' },
  })

  assert.equal(parsed.synchronized, false)
  assert.deepEqual(parsed.lines, [
    { active: true, lyric: 'First line', time: null, tlyric: undefined, rlyric: undefined },
    { active: true, lyric: 'Second line', time: null, tlyric: undefined, rlyric: undefined },
  ])
})
