# Performance architecture

This refactor deliberately retains Vue 3, Vite, Tauri 2 and the native Rodio
player. The existing framework stack is not the dominant cost: the library
already has a SQLite metadata index, incremental fingerprints, a startup
snapshot, Rayon scanning and a fixed-height virtual list. The remaining costs
were bridge frequency, checkpoint size, repeated artwork parsing and an
avoidable frontend dependency.

## Runtime boundaries

- `windowApi` remains the only WebView/Tauri bridge. Settings, playlist and
  collection startup state now arrive in one bootstrap call.
- `PlaybackClock` advances visible progress from a monotonic local clock every
  200 ms and reconciles with the native player once per second. Within one
  visual tick of the track end it resumes 200 ms native confirmation so track
  transition timing is unchanged.
- `playbackOrder` owns previous/next wrapping, shuffle ordering and end-of-track
  mode decisions as a pure, tested model.
- Playlist state uses a v4 update contract across IPC. Queue arrays are sent
  only after structural changes; routine progress, volume and index checkpoints
  contain scalars only.
- Rust reconstructs the same complete v3 restore object. On disk,
  `last-playlist.json` is the low-frequency queue structure and
  `last-playlist-state.json` is the high-frequency scalar checkpoint.
  `structureRevision` prevents a checkpoint from a previous queue being
  applied after a structural change.
- UI cover access and MPRIS metadata share one 64-entry LRU artwork cache.
  Cache entries use source size and modification time for invalidation, include
  negative results, and reuse the existing bounded 32-file disk cache.

## Measured results

Measurements use the same container, npm cache and production build command for
baseline `09018173` and this branch. Build time is the median of eight warm
runs; two baseline outliers caused by shared-container contention are retained
in the raw observations but do not affect the median.

| Metric | Baseline | Refactor |
| --- | ---: | ---: |
| Production JS + CSS | 451,621 B | 441,923 B |
| Production JS + CSS gzip | 147,727 B | 144,086 B |
| Production JavaScript | 369,188 B | 359,490 B |
| Production JavaScript gzip | 132,657 B | 129,016 B |
| Warm production build median | 2.98 s | 3.00 s |
| Startup state IPC calls | 3 | 1 |
| Playing status IPC, steady state | 5/s | 1/s |
| 10,000-track routine checkpoint over IPC | 1,558,003 B | 213 B |
| 10,000-track routine checkpoint written to disk | full queue | 246 B pretty JSON |

The full queue is still serialized once when its structure changes. Routine
checkpoint reduction is therefore not achieved by dropping queue data or
changing playback semantics.

## Compatibility scope

No `.vue`, CSS, font, image, icon, window geometry or animation declaration is
changed by the refactor. Tests lock representative product motion timing and
easing, all four playback modes, shuffle behavior, audio formats, embedded
lyrics priority, offline operation, MPRIS/D-Bus wiring and Wayland window
behavior.

The current container cannot compile or launch Tauri because it lacks
`pkg-config` and GLib/GTK/WebKitGTK/ALSA development packages. Pure Rust playlist
and artwork modules are compiled and tested in isolation. Native niri,
fractional scaling, media keys, MPRIS, file dialogs, memory, idle CPU, scanning
throughput, large-library scrolling and screenshot diff remain manual
verification items on a Fedora Wayland host.
