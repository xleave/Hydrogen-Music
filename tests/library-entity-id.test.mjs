import test from 'node:test'
import assert from 'node:assert/strict'
import { libraryEntityId } from '../src/utils/libraryEntityId.mjs'

test('artist and album routes use stable compact ids for long Unicode metadata', () => {
  const longArtist = '歌手'.repeat(2048)
  const first = libraryEntityId('artist', [longArtist])
  const second = libraryEntityId('artist', [longArtist])

  assert.equal(first, second)
  assert.match(first, /^artist:[0-9a-f]{16}$/)
})

test('albums with the same title but different album artists remain distinct', () => {
  const first = libraryEntityId('album', ['Artist A', 'Shared title'])
  const second = libraryEntityId('album', ['Artist B', 'Shared title'])

  assert.notEqual(first, second)
  assert.match(first, /^album:[0-9a-f]{16}$/)
  assert.match(second, /^album:[0-9a-f]{16}$/)
})
