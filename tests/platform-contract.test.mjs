import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
}

test('the supported local audio format set remains intact', () => {
  const library = source('src-tauri/src/library.rs')
  const cargo = source('src-tauri/Cargo.toml')

  for (const extension of [
    'mp3', 'flac', 'wav', 'aac', 'm4a', 'ogg', 'opus', 'wma',
    'ape', 'alac', 'aiff', 'mp2', 'mpc', 'wv', 'speex',
  ]) {
    assert.match(library, new RegExp(`"${extension}"`))
  }
  assert.match(cargo, /rodio = \{ version = "0\.22\.2", features = \["symphonia-all"\] \}/)
})

test('embedded lyrics keep priority over the same-name sidecar lrc file', () => {
  const backend = source('src-tauri/src/lib.rs')
  const embedded = backend.indexOf('ItemKey::Lyrics')
  const sidecar = backend.indexOf('with_extension("lrc")')

  assert.ok(embedded >= 0)
  assert.ok(sidecar > embedded)
  assert.match(backend.slice(embedded, sidecar), /return Ok\(Some\(lyrics\.to_owned\(\)\)\)/)
})

test('production runtime remains local-only without a network service', () => {
  const noNetwork = source('src/platform/noNetwork.js')
  const vite = source('vite.config.js')
  const tauri = JSON.parse(source('src-tauri/tauri.conf.json'))

  assert.match(noNetwork, /本地版不提供网络请求/)
  assert.match(vite, /find: 'axios', replacement: resolve\(__dirname, '\.\/src\/platform\/noNetwork\.js'\)/)
  assert.match(tauri.app.security.csp, /connect-src 'self' ipc: http:\/\/ipc\.localhost/)
  assert.doesNotMatch(tauri.app.security.csp, /connect-src[^;]*https:/)
})

test('Linux media controls use MPRIS over D-Bus without an X11 window handle', () => {
  const cargo = source('src-tauri/Cargo.toml')
  const media = source('src-tauri/src/media.rs')

  assert.match(cargo, /target_os = "linux", target_os = "windows"/)
  assert.match(cargo, /souvlaki = \{ version = "0\.8\.3", default-features = false, features = \["use_zbus"\] \}/)
  assert.match(media, /#\[cfg\(not\(target_os = "windows"\)\)\]\s*let hwnd = None/)
  for (const action of ['play', 'pause', 'stop', 'toggle', 'next', 'previous', 'seek', 'volume']) {
    assert.match(media, new RegExp(`"${action}"`))
  }
})

test('Wayland native window and close-to-tray behavior remain explicit', () => {
  const main = source('src-tauri/src/main.rs')
  const backend = source('src-tauri/src/lib.rs')
  const tauri = JSON.parse(source('src-tauri/tauri.conf.json'))

  assert.match(main, /std::env::set_var\("GDK_BACKEND", "wayland"\)/)
  assert.doesNotMatch(main, /GDK_BACKEND", "x11"/)
  assert.equal(tauri.app.windows[0].decorations, false)
  assert.equal(tauri.app.windows[0].width, 1024)
  assert.equal(tauri.app.windows[0].height, 672)
  assert.match(backend, /CloseRequested \{ api, \.\. \}/)
  assert.match(backend, /api\.prevent_close\(\)/)
  assert.match(backend, /window\.hide\(\)/)
})
