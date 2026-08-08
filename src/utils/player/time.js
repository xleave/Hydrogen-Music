import dayjs from 'dayjs'
import duration from 'dayjs/plugin/duration'

dayjs.extend(duration)

export function songTime(value) {
  if (value === 0 || value === '--') return value
  if (!value) return undefined

  const parsed = dayjs.duration(value)
  return `${parsed.minutes()}:${parsed.seconds().toString().padStart(2, '0')}`
}

export function songTime2(value) {
  const totalSeconds = Number(value) || 0
  const minutes = Math.floor(totalSeconds / 60).toString().padStart(2, '0')
  const seconds = Math.floor(totalSeconds % 60).toString().padStart(2, '0')
  return `${minutes}:${seconds}`
}
