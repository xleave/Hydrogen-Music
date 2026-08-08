let enabled = false
let recent = []

function nowMs() {
  return globalThis.performance?.now?.() ?? Date.now()
}

export function setRendererPerformanceMonitoring(value) {
  enabled = Boolean(value)
  recent = []
}

export function startRendererSpan() {
  return enabled ? nowMs() : null
}

export function recordRendererSpan(name, startedAt, detail = null) {
  if (!enabled || startedAt == null) return
  const durationMs = Math.max(0, nowMs() - startedAt)
  recent.push({
    name,
    durationMs,
    detail,
    finishedAt: Date.now(),
  })
  if (recent.length > 24) recent.splice(0, recent.length - 24)
}

export function recordRendererSpanAfterPaint(name, startedAt, detail = null) {
  if (!enabled || startedAt == null) return
  requestAnimationFrame(() => {
    requestAnimationFrame(() => recordRendererSpan(name, startedAt, detail))
  })
}

export function getRendererPerformanceMetrics() {
  return {
    recent: recent.map((item) => ({ ...item })),
  }
}
