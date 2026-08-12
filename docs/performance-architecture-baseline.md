# Performance architecture baseline

Baseline commit: `09018173ae2660ac172114b1f3d25f4df3f3ab2c`

The current Vue 3, Vite and Tauri 2 stack is retained. The library already uses a
SQLite metadata index, a cached startup snapshot, bounded parallel metadata
parsing and a fixed-height virtual list. Replacing the framework would add
migration risk without addressing the measured remaining costs.

## Reproducible measurements

- Node.js `v24.14.0`, npm `11.9.0`
- Rust `1.97.1` (`stable-x86_64-unknown-linux-gnu`)
- `npm ci`: completed
- `npm run test:frontend`: 27 tests passed
- `npm run build`: 3.29 seconds wall time
- production entry JS: 131,496 bytes (51,714 bytes gzip)
- dayjs duration chunk: 43,063 bytes (15,285 bytes gzip)
- a deterministic 10,000-track v3 playlist checkpoint: measured by
  `npm run measure:performance`
- playback status bridge calls while playing: 5 per second by source contract
  (`200ms` polling interval)
- startup state bridge calls: three (`settings`, `last playlist`, `collections`)

## Environment limits

The container does not provide `pkg-config`, GLib/GTK/WebKitGTK or ALSA
development headers. Rust/Tauri compilation and a native WebView launch are
therefore blocked before application code is compiled. Native Wayland/niri,
MPRIS, memory, idle CPU and screenshot-diff verification remain explicit manual
checks. Source-level UI and motion contracts are covered by frontend tests.
