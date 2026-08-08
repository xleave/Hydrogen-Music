<script setup>
defineProps({
  active: { type: Boolean, default: false },
  label: { type: String, required: true },
})
defineEmits(['toggle'])
</script>

<template>
  <div class="setting-toggle" @click="$emit('toggle')">
    <div class="toggle-off" :class="{ 'toggle-on-in': active }">{{ label }}</div>
    <Transition name="setting-toggle-slide">
      <div v-show="active" class="toggle-on"></div>
    </Transition>
  </div>
</template>

<style scoped lang="scss">
.setting-toggle {
  margin-right: 1px;
  width: 200px;
  height: 34px;
  position: relative;
  overflow: hidden;
  cursor: pointer;

  .toggle-on,
  .toggle-off {
    padding: 5px 10px;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    font: 13px SourceHanSansCN-Bold;
    line-height: 24px;
    transition: .2s;
  }

  .toggle-off {
    position: relative;
    z-index: 1;
    background-color: rgba(255, 255, 255, .35);
  }

  .toggle-on {
    position: absolute;
    z-index: 0;
    top: 0;
    left: 0;
    background-color: black;
  }

  .toggle-on-in {
    color: white;
    background-color: transparent;
  }
}

.setting-toggle-slide-enter-active,
.setting-toggle-slide-leave-active { transition: .1s; }
.setting-toggle-slide-enter-from,
.setting-toggle-slide-leave-to { transform: translateX(-100%); }
</style>
