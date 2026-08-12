function normalized(value, fallback = 0) {
  const number = Number(value)
  return Number.isFinite(number) ? Math.max(0, number) : fallback
}

export class PlaybackClock {
  constructor(status, now = 0) {
    this.apply(status, now)
  }

  apply(status, now) {
    this.duration = normalized(status.duration)
    this.position = Math.min(normalized(status.position), this.duration)
    this.playing = Boolean(status.playing) && !status.ended
    this.syncedAt = normalized(now)
    return status
  }

  estimate(now) {
    if (!this.playing) return this.position
    const elapsed = Math.max(0, normalized(now) - this.syncedAt) / 1_000
    return Math.min(this.duration, this.position + elapsed)
  }

  shouldReconcile(now, intervalMs) {
    return normalized(now) - this.syncedAt >= intervalMs
  }

  needsEndConfirmation(now, leadSeconds) {
    return this.playing
      && this.duration > 0
      && this.estimate(now) >= Math.max(0, this.duration - leadSeconds)
  }
}
