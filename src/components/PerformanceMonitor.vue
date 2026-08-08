<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useLocalStore } from '../store/localStore'
import { usePlayerStore } from '../store/playerStore'

const route = useRoute()
const localStore = useLocalStore()
const playerStore = usePlayerStore()
const { currentSelectedSongs } = storeToRefs(localStore)
const { songList } = storeToRefs(playerStore)

const expanded = ref(false)
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
})

let frameHandle = null
let sampleTimer = null
let lastFrame = 0
let frameDurations = []
let previousIpcTotal = 0
let previousAudioStatus = 0
let previousSampleAt = 0

const visible = computed(() => route.name === 'settings')
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
  }

  previousIpcTotal = totalCalls
  previousAudioStatus = audioStatusCalls
}

function startMonitoring() {
  if (frameHandle !== null) return
  windowApi.setPerformanceMonitoring?.(true)
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
}

function toggleMonitor() {
  expanded.value = !expanded.value
}

watch(expanded, (value) => {
  if (value && visible.value) startMonitoring()
  else stopMonitoring()
})

watch(visible, (value) => {
  if (!value) {
    expanded.value = false
    stopMonitoring()
  }
})

onBeforeUnmount(stopMonitoring)
</script>

<template>
  <div v-if="visible" class="performance-monitor" :class="{ 'performance-monitor-open': expanded }">
    <button class="monitor-header" type="button" @click="toggleMonitor">
      <span>性能监测</span>
      <span class="monitor-state">{{ expanded ? '收起' : '展开' }}</span>
    </button>

    <div v-if="expanded" class="monitor-body">
      <div class="monitor-note">仅展开时采样；用于定位 renderer / IPC / 数据规模热点。</div>

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
.performance-monitor {
  position: fixed;
  right: 9.5%;
  bottom: 22Px;
  z-index: 1200;
  width: 190Px;
  background: rgba(225, 240, 240, .96);
  box-shadow: 0 0 0 .5Px rgba(0, 0, 0, .24), 0 8Px 24Px rgba(0, 0, 0, .08);
  transition: width .25s cubic-bezier(.19,.8,.49,.99);

  &.performance-monitor-open { width: 430Px; }

  .monitor-header {
    width: 100%;
    height: 34Px;
    padding: 0 12Px;
    border: 0;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: black;
    font: 12Px SourceHanSansCN-Bold;
    cursor: pointer;
    text-align: left;

    .monitor-state {
      font: 10Px SourceHanSansCN-Bold;
      color: rgba(0, 0, 0, .46);
    }
  }

  .monitor-body {
    padding: 0 12Px 12Px;
    max-height: min(560Px, 72vh);
    overflow: auto;
    text-align: left;
    &::-webkit-scrollbar { display: none; }
  }

  .monitor-note {
    padding: 7Px 0 10Px;
    border-top: .5Px solid rgba(0, 0, 0, .14);
    font: 9Px SourceHanSansCN-Bold;
    color: rgba(0, 0, 0, .48);
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
