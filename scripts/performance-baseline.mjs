import { readdirSync, statSync } from 'node:fs'
import { extname } from 'node:path'

import { buildPlaylistCheckpoint } from '../src/utils/player/playlistCheckpoint.mjs'

const assetDirectory = new URL('../dist/assets/', import.meta.url)
const assets = readdirSync(assetDirectory)
  .map((name) => ({ name, bytes: statSync(new URL(name, assetDirectory)).size }))
const codeAndStyles = assets.filter(({ name }) => ['.js', '.css'].includes(extname(name)))

const ids = Array.from({ length: 10_000 }, (_, index) =>
  `track:${index.toString(16).padStart(16, '0')}:${'/music/library/'.padEnd(48, 'x')}${index}`)
const playlist = JSON.stringify({
  version: 3,
  songIds: ids,
  shuffledSongIds: [...ids].reverse(),
  currentSongId: ids[5_000],
  currentIndex: 5_000,
  shuffleIndex: 4_999,
  progress: 183.25,
  volume: 0.3,
  playMode: 3,
})
const playlistState = {
  structureRevision: 1,
  songIds: ids,
  shuffledSongIds: [...ids].reverse(),
  currentSongId: ids[5_000],
  currentIndex: 5_000,
  shuffleIndex: 4_999,
  progress: 183.25,
  volume: 0.3,
  playMode: 3,
}
const structuralCheckpoint = buildPlaylistCheckpoint(playlistState, true)
const scalarCheckpoint = buildPlaylistCheckpoint(playlistState, false)

const report = {
  productionAssets: {
    codeAndStylesBytes: codeAndStyles.reduce((sum, asset) => sum + asset.bytes, 0),
    javascriptBytes: codeAndStyles
      .filter(({ name }) => extname(name) === '.js')
      .reduce((sum, asset) => sum + asset.bytes, 0),
    cssBytes: codeAndStyles
      .filter(({ name }) => extname(name) === '.css')
      .reduce((sum, asset) => sum + asset.bytes, 0),
    largest: codeAndStyles.sort((left, right) => right.bytes - left.bytes).slice(0, 8),
  },
  playlistCheckpoint: {
    tracks: ids.length,
    legacyV3Bytes: Buffer.byteLength(playlist),
    structuralV4Bytes: Buffer.byteLength(structuralCheckpoint.payload),
    scalarV4Bytes: Buffer.byteLength(scalarCheckpoint.payload),
  },
}

process.stdout.write(`${JSON.stringify(report, null, 2)}\n`)
