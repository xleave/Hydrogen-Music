<script setup>
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { storeToRefs } from 'pinia'
import SettingToggle from './settings/SettingToggle.vue'
import { useLocalStore } from '../store/localStore'
import { usePlayerStore } from '../store/playerStore'
import {
  getRendererPerformanceMetrics,
  setRendererPerformanceMonitoring,
} from '../utils/performanceMonitor'

const MONITOR_STORAGE_KEY = 'hydrogen.performanceMonitor.enabled'
const MONITOR_POSITION_KEY = 'hydrogen.performanceMonitor.position'

const route = useRoute()
const localStore = useLocalStore()
const playerStore = usePlayerStore()
const { currentSelectedSongs } = storeToRefs(localStore)
const { songList } = storeToRefs(playerStore)

function readMonitorEnabled() {
  try {
    return localStorage.getItem(MONITOR_STORAGE_KEY) === '1'
  } catch {
    return false
  }
}

function persistMonitorEnabled(value) {
  try {
    localStorage.setItem(MONITOR_STORAGE_KEY, value ? '1' : '0')
  } catch {
    // Diagnostics must never affect normal application behavior.
  }
}

function readMonitorPosition() {
  try {
    const value = JSON.parse(localStorage.getItem(MONITOR_POSITION_KEY) || 'null')
    if (!Number.isFinite(value?.x) || !Number.isFinite(value?.y)) return null
    return { x: value.x, y: value.y }
  } catch {
    return null
  }
}

function persistMonitorPosition(value) {
  try {
    localStorage.setItem(MONITOR_POSITION_KEY, JSON.stringify(value))
  } catch {
    // Diagnostics must never affect normal application behavior.
  }
}

const enabled = ref(readMonitorEnabled())
const settingsVisible = computed(() => route.name === 'settings')
const settingsTarget = ref(null)
const monitorPanel = ref(null)
const panelPosition = ref(readMonitorPosition())
const panelStyle = computed(() => panelPosition.value
  ? {
      left: `${panelPosition.value.x}px`,
      top: `${panelPosition.value.y}px`,
      right: 'auto',
      bottom: 'auto',
    }
  : {})
const metrics = ref({
  fps: 0,
  averageFrame: 0,
  p95Frame: 0,
  maxFrame: 0,
  over16: 0,
  over33: 0,
  domNodes: 0,
  virtualRows: 0,
  lyricRows: 0,
  ipcRate: 0,
  ipcActive: 0,
  ipcMaxActive: 0,
  audioStatusRate: 0,
  jsHeap: null,
  recentSlow: [],
  recentRenderer: [],
})

let frameHandle = null
let sampleTimer = null
let lastFrame = 0
let frameDurations = []
let previousIpcTotal = 0
let previousAudioStatus = 0
let previousSampleAt = 0
let targetRevision = 0
let dragState = null

const selectedCount = computed(() => currentSelectedSongs.value?.length || 0)
const queueCount = computed(() => songList.value?.length || 0)

function percentile(values, ratio) {
  if (!values.length) return 0
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * ratio))]
}

function frameLoop(timestamp) {
  if (lastFrame > 0) {
    const duration = timestamp - lastFrame
    if (duration > 0 && duration < 1000) {
      frameDurations.push(duration)
      if (frameDurations.length > 360) frameDurations.splice(0, frameDurations.length - 360)
    }
  }
  lastFrame = timestamp
  frameHandle = requestAnimationFrame(frameLoop)
}

function formatBytes(value) {
  if (!Number.isFinite(value)) return '—'
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`
  return `${(value / 1024 / 1024).toFixed(1)} MiB`
}

function findOtherSettingsTarget() {
  const sections = document.querySelectorAll('.settings-page .settings-item')
  for (const section of sections) {
    if (section.querySelector('.item-title')?.textContent?.trim() !== '其他') continue
    return section.querySelector('.item-options')
  }
  return null
}

async function syncSettingsTarget() {
  const revision = ++targetRevision
  settingsTarget.value = null
  if (!settingsVisible.value) return

  await nextTick()
  if (revision !== targetRevision || !settingsVisible.value) return
  settingsTarget.value = findOtherSettingsTarget()

  if (!settingsTarget.value) {
    requestAnimationFrame(() => {
      if (revision !== targetRevision || !settingsVisible.value) return
      settingsTarget.value = findOtherSettingsTarget()
    })
  }
}

function updateMetrics() {
  const now = performance.now()
  const elapsed = previousSampleAt > 0 ? Math.max(0.001, (now - previousSampleAt) / 1000) : 1
  previousSampleAt = now

  const durations = frameDurations
  frameDurations = []
  const averageFrame = durations.length
    ? durations.reduce((sum, value) => sum + value, 0) / durations.length
    : 0

  const ipc = windowApi.getPerformanceMetrics?.() || {}
  const renderer = getRendererPerformanceMetrics()
  const byCommand = ipc.byCommand || {}
  const totalCalls = Number(ipc.totalCalls || 0)
  const audioStatusCalls = Number(byCommand.audio_status?.calls || 0)
  const jsMemory = performance.memory?.usedJSHeapSize

  metrics.value = {
    fps: averageFrame > 0 ? Math.min(999, 1000 / averageFrame) : 0,
    averageFrame,
    p95Frame: percentile(durations, 0.95),
    maxFrame: durations.length ? Math.max(...durations) : 0,
    over16: durations.filter((value) => value > 16.7).length,
    over33: durations.filter((value) => value > 33.3).length,
    domNodes: document.getElementsByTagName('*').length,
    virtualRows: document.querySelectorAll('.vue-recycle-scroller__item-view').length,
    lyricRows: document.querySelectorAll('.lyric-line').length,
    ipcRate: Math.max(0, totalCalls - previousIpcTotal) / elapsed,
    ipcActive: Number(ipc.activeCalls || 0),
    ipcMaxActive: Number(ipc.maxActiveCalls || 0),
    audioStatusRate: Math.max(0, audioStatusCalls - previousAudioStatus) / elapsed,
    jsHeap: Number.isFinite(jsMemory) ? jsMemory : null,
    recentSlow: (ipc.recent || [])
      .filter((item) => item.durationMs >= 8)
      .slice(-6)
      .reverse(),
    recentRenderer: (renderer.recent || [])
      .filter((item) => item.durationMs >= 4)
      .slice(-5)
      .reverse(),
  }

  previousIpcTotal = totalCalls
  previousAudioStatus = audioStatusCalls
}

function startMonitoring() {
  if (frameHandle !== null) return
  windowApi.setPerformanceMonitoring?.(true)
  setRendererPerformanceMonitoring(true)
  previousIpcTotal = 0
  previousAudioStatus = 0
  previousSampleAt = performance.now()
  lastFrame = 0
  frameDurations = []
  frameHandle = requestAnimationFrame(frameLoop)
  sampleTimer = setInterval(updateMetrics, 750)
  updateMetrics()
}

function stopMonitoring() {
  if (frameHandle !== null) cancelAnimationFrame(frameHandle)
  frameHandle = null
  if (sampleTimer !== null) clearInterval(sampleTimer)
  sampleTimer = null
  lastFrame = 0
  frameDurations = []
  windowApi.setPerformanceMonitoring?.(false)
  setRendererPerformanceMonitoring(false)
}

function toggleMonitoring() {
  enabled.value = !enabled.value
}

function clampPanelPosition(x, y) {
  const panel = monitorPanel.value
  if (!panel) return { x, y }
  const margin = 8
  const maxX = Math.max(margin, window.innerWidth - panel.offsetWidth - margin)
  const maxY = Math.max(margin, window.innerHeight - panel.offsetHeight - margin)
  return {
    x: Math.min(maxX, Math.max(margin, x)),
    y: Math.min(maxY, Math.max(margin, y)),
  }
}

function onPanelPointerMove(event) {
  if (!dragState) return
  panelPosition.value = clampPanelPosition(
    dragState.startX + event.clientX - dragState.pointerX,
    dragState.startY + event.clientY - dragState.pointerY,
  )
}

function stopPanelDrag() {
  if (!dragState) return
  dragState = null
  window.removeEventListener('pointermove', onPanelPointerMove)
  window.removeEventListener('pointerup', stopPanelDrag)
  window.removeEventListener('pointercancel', stopPanelDrag)
  if (panelPosition.value) persistMonitorPosition(panelPosition.value)
}

function startPanelDrag(event) {
  if (event.button !== 0 || !monitorPanel.value) return
  event.preventDefault()
  const rect = monitorPanel.value.getBoundingClientRect()
  dragState = {
    pointerX: event.clientX,
    pointerY: event.clientY,
    startX: rect.left,
    startY: rect.top,
  }
  panelPosition.value = { x: rect.left, y: rect.top }
  window.addEventListener('pointermove', onPanelPointerMove)
  window.addEventListener('pointerup', stopPanelDrag)
  window.addEventListener('pointercancel', stopPanelDrag)
}

watch(enabled, (value) => {
  persistMonitorEnabled(value)
  if (value) startMonitoring()
  else stopMonitoring()
}, { immediate: true })

watch(settingsVisible, syncSettingsTarget, { immediate: true, flush: 'post' })

onBeforeUnmount(() => {
  targetRevision += 1
  stopPanelDrag()
  stopMonitoring()
})
</script>

<template>
  <Teleport v-if="settingsTarget" :to="settingsTarget">
    <div class="monitor-setting-control">
      <div class="monitor-setting-label">性能监测</div>
      <div class="monitor-setting-operation">
        <SettingToggle
          :active="enabled"
          :label="enabled ? '已开启' : '已关闭'"
          @toggle="toggleMonitoring"
        />
      </div>
    </div>
  </Teleport>

  <div v-if="enabled" ref="monitorPanel" class="performance-monitor" :style="panelStyle">
    <div class="monitor-header" @pointerdown="startPanelDrag">性能监测</div>

    <div class="monitor-body">
      <div class="metric-section">
        <div class="metric-title">FRAME</div>
        <div class="metric-grid">
          <div><span>FPS</span><b>{{ metrics.fps.toFixed(1) }}</b></div>
          <div><span>平均帧</span><b>{{ metrics.averageFrame.toFixed(1) }} ms</b></div>
          <div><span>P95</span><b>{{ metrics.p95Frame.toFixed(1) }} ms</b></div>
          <div><span>最慢帧</span><b>{{ metrics.maxFrame.toFixed(1) }} ms</b></div>
          <div><span>&gt;16.7ms</span><b>{{ metrics.over16 }}</b></div>
          <div><span>&gt;33.3ms</span><b>{{ metrics.over33 }}</b></div>
        </div>
      </div>

      <div class="metric-section">
        <div class="metric-title">RENDERER</div>
        <div class="metric-grid">
          <div><span>DOM</span><b>{{ metrics.domNodes }}</b></div>
          <div><span>虚拟行</span><b>{{ metrics.virtualRows }}</b></div>
          <div><span>歌词 DOM</span><b>{{ metrics.lyricRows }}</b></div>
          <div><span>当前列表</span><b>{{ selectedCount }}</b></div>
          <div><span>播放队列</span><b>{{ queueCount }}</b></div>
          <div><span>JS Heap</span><b>{{ formatBytes(metrics.jsHeap) }}</b></div>
        </div>
        <div v-if="metrics.recentRenderer.length" class="slow-list">
          <div class="slow-title">最近 Renderer 任务</div>
          <div v-for="(item, index) in metrics.recentRenderer" :key="`${item.name}-${item.finishedAt}-${index}`" class="slow-row">
            <span>{{ item.name }}</span>
            <b>{{ item.durationMs.toFixed(1) }} ms</b>
          </div>
        </div>
      </div>

      <div class="metric-section">
        <div class="metric-title">IPC</div>
        <div class="metric-grid">
          <div><span>调用速率</span><b>{{ metrics.ipcRate.toFixed(1) }}/s</b></div>
          <div><span>audio_status</span><b>{{ metrics.audioStatusRate.toFixed(1) }}/s</b></div>
          <div><span>并发</span><b>{{ metrics.ipcActive }}</b></div>
          <div><span>峰值并发</span><b>{{ metrics.ipcMaxActive }}</b></div>
        </div>
        <div v-if="metrics.recentSlow.length" class="slow-list">
          <div class="slow-title">最近慢 IPC</div>
          <div v-for="(item, index) in metrics.recentSlow" :key="`${item.command}-${item.finishedAt}-${index}`" class="slow-row">
            <span>{{ item.command }}</span>
            <b>{{ item.durationMs.toFixed(1) }} ms</b>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.monitor-setting-control {
  margin-bottom: 32px;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;

  .monitor-setting-label {
    font-family: SourceHanSansCN-Bold;
    font-size: 16px;
    color: black;
    text-align: left;
  }
}

.performance-monitor {
  position: fixed;
  right: 24Px;
  bottom: 24Px;
  z-index: var(--z-diagnostic, 1200);
  width: 430Px;
  background: rgba(225, 240, 240, .78);
  box-shadow: 0 0 0 .5Px rgba(0, 0, 0, .18), 0 6Px 20Px rgba(0, 0, 0, .06);

  .monitor-header {
    width: 100%;
    height: 34Px;
    padding: 0 12Px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    color: black;
    font: 12Px SourceHanSansCN-Bold;
    text-align: left;
    cursor: move;
    user-select: none;
    touch-action: none;
  }

  .monitor-body {
    padding: 0 12Px 12Px;
    max-height: min(560Px, 72vh);
    overflow: auto;
    text-align: left;
    &::-webkit-scrollbar { display: none; }
  }

  .metric-section {
    padding: 9Px 0 4Px;
    border-top: .5Px solid rgba(0, 0, 0, .12);
  }

  .metric-title {
    margin-bottom: 7Px;
    font: 9Px Bender-Bold;
    letter-spacing: .12em;
    color: rgba(0, 0, 0, .42);
  }

  .metric-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 5Px 16Px;

    div {
      min-width: 0;
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 10Px;
      font: 10Px SourceHanSansCN-Bold;
    }

    span { color: rgba(0, 0, 0, .52); }
    b {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font: 11Px Bender-Bold;
      color: black;
    }
  }

  .slow-list { margin-top: 9Px; }
  .slow-title {
    margin-bottom: 4Px;
    font: 9Px SourceHanSansCN-Bold;
    color: rgba(0, 0, 0, .45);
  }
  .slow-row {
    min-height: 21Px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12Px;
    border-top: .5Px solid rgba(0, 0, 0, .07);
    span { font: 10Px Bender-Bold; color: rgba(0, 0, 0, .68); }
    b { font: 10Px Bender-Bold; color: black; }
  }
}
</style>
