<script setup>
import { computed } from 'vue'

const props = defineProps({
  width: { type: [Number, String], default: null },
  padding: { type: String, default: '6px' },
  fontSize: { type: Number, default: 14 },
  hoverOpacity: { type: Number, default: 1 },
  hoverBackground: { type: String, default: 'rgba(255, 255, 255, .35)' },
})
defineEmits(['click'])

const style = computed(() => ({
  width: props.width == null ? undefined : (typeof props.width === 'number' ? `${props.width}px` : props.width),
  padding: props.padding,
  fontSize: `${props.fontSize}px`,
  '--setting-button-hover-opacity': String(props.hoverOpacity),
  '--setting-button-hover-background': props.hoverBackground,
}))
</script>

<template>
  <div class="setting-button" :style="style" @click="$emit('click', $event)">
    <slot />
  </div>
</template>

<style scoped lang="scss">
.setting-button {
  box-sizing: border-box;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(255, 255, 255, .35);
  color: black;
  font-family: SourceHanSansCN-Bold;
  line-height: 1.25;
  transition: opacity .2s, background-color .2s, box-shadow .2s;
  cursor: pointer;

  &:hover {
    opacity: var(--setting-button-hover-opacity);
    background-color: var(--setting-button-hover-background);
    box-shadow: inset 0 0 0 1px black;
  }
}
</style>
