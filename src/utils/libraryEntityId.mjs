const FNV_OFFSET = 0x811c9dc5
const FNV_REVERSE_OFFSET = 0x9e3779b9
const FNV_PRIME = 0x01000193
const encoder = new TextEncoder()

function fnv1a32(bytes, seed, reverse = false) {
  let value = seed >>> 0
  for (let index = reverse ? bytes.length - 1 : 0;
    reverse ? index >= 0 : index < bytes.length;
    index += reverse ? -1 : 1) {
    value ^= bytes[index]
    value = Math.imul(value, FNV_PRIME) >>> 0
  }
  return value.toString(16).padStart(8, '0')
}

export function libraryEntityId(type, identityParts) {
  const identity = JSON.stringify([type, ...(identityParts || []).map((part) => String(part))])
  const bytes = encoder.encode(identity)
  return `${type}:${fnv1a32(bytes, FNV_OFFSET)}${fnv1a32(bytes, FNV_REVERSE_OFFSET, true)}`
}
