import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
}

test('empty hydrated playlist does not mount player surfaces', () => {
  const store = source('src/store/playerStore.js')
  const app = source('src/App.vue')
  const myMusic = source('src/views/MyMusic.vue')

  assert.match(store, /hasPlaylist:\s*\(state\)\s*=>\s*Array\.isArray\(state\.songList\)\s*&&\s*state\.songList\.length\s*>\s*0/)
  assert.equal((app.match(/v-if="playerStore\.hasPlaylist"/g) || []).length, 2)
  assert.doesNotMatch(app, /v-if="playerStore\.songList"/)
  assert.match(myMusic, /my-music-full': !playerStore\.hasPlaylist/)
})

test('progress sliders use one semantic marker instead of duplicate ids', () => {
  const widget = source('src/components/MusicWidget.vue')
  const player = source('src/components/Player.vue')
  const lifecycle = source('src/utils/player/lifecycle.js')

  for (const content of [widget, player]) {
    assert.match(content, /data-player-progress/)
    assert.doesNotMatch(content, /id="widget-progress"/)
  }
  assert.match(lifecycle, /closest\('\[data-player-progress\]'\)/)
})

test('native volume synchronization belongs to player lifecycle', () => {
  const widget = source('src/components/MusicWidget.vue')
  const lifecycle = source('src/utils/player/lifecycle.js')

  assert.doesNotMatch(widget, /watch\(\(\) => volume\.value/)
  assert.match(lifecycle, /watch\(volume,/)
  assert.match(lifecycle, /currentMusic\.value\.volume\(normalized\)/)
})

test('settings remains a global overlay above the full-screen player', () => {
  const app = source('src/App.vue')
  const style = source('src/style.css')

  assert.match(app, /settingsOverlay = computed\(\(\) => route\.name === 'settings'\)/)
  assert.match(app, /playerStore\.widgetState \|\| settingsOverlay/)
  assert.match(app, /z-index: var\(--z-settings\)/)
  assert.match(style, /--z-settings:\s*900/)
  assert.match(style, /--z-modal:\s*2000/)
})

test('library navigation resets the real fixed virtual scroll owner', () => {
  const router = source('src/router/router.js')
  assert.match(router, /\.local-music-detail \.virtual-list/)
  assert.match(router, /scroller\.scrollTop = 0/)
})

test('folder rows and settings controls use shared components', () => {
  const localList = source('src/components/LocalMusicList.vue')
  const settings = source('src/views/Settings.vue')

  assert.match(localList, /FolderTreeNode/)
  assert.match(settings, /SettingButton/)
  assert.match(settings, /SettingToggle/)
  assert.equal(existsSync(new URL('../src/components/ChildrenFolder.vue', import.meta.url)), false)
})

test('fixed-height lists use the shared boundary-driven virtualizer', () => {
  const vite = source('vite.config.js')
  const scroller = source('src/components/FixedVirtualScroller.js')

  assert.match(vite, /find: \/\^vue-virtual-scroller\$\//)
  assert.match(scroller, /if \(next !== firstVisible\.value\) firstVisible\.value = next/)
  assert.match(scroller, /translate3d\(0, \$\{offsetY\.value\}px, 0\)/)
})

test('lyric return can be interrupted without losing the visual offset', () => {
  const lyric = source('src/components/Lyric.vue')
  assert.match(lyric, /new DOMMatrixReadOnly\(transform\)\.m42/)
  assert.match(lyric, /if \(isReturning\) cancelReturnAnimation\(true\)/)
})
