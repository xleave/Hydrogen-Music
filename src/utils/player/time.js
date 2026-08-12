export function songTime(value) {
  if (value === 0 || value === '--') return value
  if (!value) return undefined

  const totalSeconds = Math.floor(Number(value) / 1_000)
  const minutes = Math.floor(totalSeconds / 60) % 60
  const seconds = totalSeconds % 60
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

export function songTime2(value) {
  const totalSeconds = Number(value) || 0
  const minutes = Math.floor(totalSeconds / 60).toString().padStart(2, '0')
  const seconds = Math.floor(totalSeconds % 60).toString().padStart(2, '0')
  return `${minutes}:${seconds}`
}
