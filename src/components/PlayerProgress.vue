<script setup>
import { computed, onBeforeUnmount, ref } from 'vue'

const props = defineProps({
  modelValue: { type: Number, default: 0 },
  max: { type: Number, default: 0 },
})

const emit = defineEmits(['update:modelValue'])
const root = ref(null)
const dragging = ref(false)

const ratio = computed(() => {
  const max = Math.max(0, Number(props.max) || 0)
  if (max <= 0) return 0
  return Math.max(0, Math.min(1, (Number(props.modelValue) || 0) / max))
})

const fillStyle = computed(() => ({
  transform: `scaleX(${ratio.value})`,
  transition: dragging.value ? 'none' : 'transform 220ms linear',
}))

function valueFromPointer(event) {
  const element = root.value
  const max = Math.max(0, Number(props.max) || 0)
  if (!element || max <= 0) return 0
  const rect = element.getBoundingClientRect()
  if (rect.width <= 0) return 0
  const local = Math.max(0, Math.min(rect.width, event.clientX - rect.left))
  return (local / rect.width) * max
}

function updateFromPointer(event) {
  emit('update:modelValue', valueFromPointer(event))
}

function onMouseMove(event) {
  if (!dragging.value) return
  updateFromPointer(event)
}

function stopDrag() {
  if (!dragging.value) return
  dragging.value = false
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', stopDrag)
}

function startDrag(event) {
  if (event.button !== 0) return
  event.preventDefault()
  dragging.value = true
  updateFromPointer(event)
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', stopDrag)
}

onBeforeUnmount(stopDrag)
</script>

<template>
  <div ref="root" class="player-progress" @mousedown="startDrag">
    <div class="player-progress-fill" :style="fillStyle"></div>
  </div>
</template>

<style scoped>
.player-progress {
  position: relative;
  overflow: hidden;
  cursor: pointer;
  contain: layout paint style;
  isolation: isolate;
}

.player-progress-fill {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  background: black;
  transform-origin: left center;
  backface-visibility: hidden;
  pointer-events: none;
  will-change: transform;
}
</style>
