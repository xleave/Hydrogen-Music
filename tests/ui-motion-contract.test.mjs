import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
}

const motionContracts = {
  'src/App.vue': [
    'animation: mainWindows-starting 0.8s cubic-bezier(.14,.91,.58,1) forwards',
    'transition: transform 0.5s cubic-bezier(.14,.91,.58,1)',
  ],
  'src/views/MusicPlayer.vue': [
    'animation: player-in 0.7s 0.2s cubic-bezier(0.4, 0, 0.12, 1) forwards',
    'animation: player-hide 0.4s cubic-bezier(.3,.79,.55,.99) forwards',
  ],
  'src/views/MyMusic.vue': [
    'transition: 0.5s cubic-bezier(.19,.8,.49,.99)',
    'animation: nodata-open1 0.6s cubic-bezier(.32,.81,.56,.98) forwards',
  ],
  'src/components/Player.vue': [
    'animation: cover-in 0.3s 0.65s cubic-bezier(0.4, 0, 0.12, 1) forwards',
    'transition: 0.3s cubic-bezier(.22,.89,.58,.99)',
  ],
  'src/components/Lyric.vue': [
    'transition: transform 0.58s cubic-bezier(.4,0,.12,1)',
    'animation: diamond-rotate 1.6s 0.6s cubic-bezier(.30,0,.12,1) infinite',
  ],
  'src/components/ContextMenu.vue': [
    'animation: menu-in .2s cubic-bezier(.3,.79,.55,.99) forwards',
  ],
  'src/components/GlobalDialog.vue': [
    'animation: dialog-container-in 0.4s 0.15s forwards',
    'animation: dialog-container-in 0.4s reverse',
  ],
  'src/components/GlobalNotice.vue': [
    'animation: notice-in 0.18s cubic-bezier(0.3, 0.79, 0.55, 0.99) forwards',
    'animation: notice-out 0.2s forwards',
  ],
  'src/components/PlayerProgress.vue': [
    "transition: dragging.value ? 'none' : 'transform 220ms linear'",
  ],
}

test('product motion timings and easing remain unchanged', () => {
  for (const [path, declarations] of Object.entries(motionContracts)) {
    const content = source(path)
    for (const declaration of declarations) {
      assert.ok(content.includes(declaration), `${path} lost motion contract: ${declaration}`)
    }
  }
})
