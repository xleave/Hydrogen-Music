<template>
  <div class="selector" ref="select" @click="changeOptionsVisible">
    <div class="selector-head">
      <span class="select-head-cont" :class="{ 'long-label': isLongLabel(current?.label) }">
        {{ current?.label || '请选择' }}
      </span>
    </div>
    <teleport to="body">
      <transition name="selector" @enter="absolutePosition(overlay, select)">
        <div
          class="selector-option"
          :style="{
            '--count': visibleCount,
            maxHeight: optionMaxHeight,
          }"
          v-if="option"
          ref="overlay"
          @click.stop
        >
          <div class="selector-search" v-if="searchable">
            <input
              ref="searchInput"
              v-model="search"
              :placeholder="searchPlaceholder"
              @keydown.stop
              @click.stop
            >
          </div>
          <div
            class="selector-option-item"
            v-for="item in filteredOptions"
            :key="String(item.value)"
            @click.stop="changeOption(item)"
            :class="{
              'selector-option-item-selected': modelValue === item.value,
            }"
          >
            <span :class="{'long-label': isLongLabel(item?.label)}">{{ item?.label }}</span>
          </div>
          <div class="selector-empty" v-if="!filteredOptions.length">无匹配项</div>
        </div>
      </transition>
    </teleport>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { absolutePosition } from '../utils/domHandler'

const props = defineProps({
  options: {
    type: Array,
    default: () => [],
  },
  modelValue: null,
  maxItems: {
    type: Number,
    default: 4,
  },
  searchable: {
    type: Boolean,
    default: false,
  },
  searchPlaceholder: {
    type: String,
    default: '搜索',
  },
})

const emit = defineEmits(['update:modelValue'])
const select = ref()
const overlay = ref()
const searchInput = ref()
const option = ref(false)
const search = ref('')

const current = computed(() => props.options.find((x) => x.value === props.modelValue))
const filteredOptions = computed(() => {
  const keyword = search.value.trim().toLocaleLowerCase()
  if (!props.searchable || !keyword) return props.options
  return props.options.filter((item) => String(item?.label ?? '').toLocaleLowerCase().includes(keyword))
})
const visibleCount = computed(() => Math.min(filteredOptions.value.length || 1, props.maxItems))
const optionMaxHeight = computed(() => {
  const searchHeight = props.searchable ? 46 : 0
  return `${props.maxItems * 34 + 16 + searchHeight}px`
})

function changeOption(item) {
  emit('update:modelValue', item.value)
  option.value = false
  search.value = ''
}

function isLongLabel(label) {
  return label?.length >= 20
}

function clickOutside(event) {
  const inSelect = select.value?.contains(event.target)
  const inOverlay = overlay.value?.contains(event.target)
  if (!inSelect && !inOverlay) option.value = false
}

async function changeOptionsVisible() {
  option.value = !option.value
  if (!option.value) {
    search.value = ''
    return
  }
  await nextTick()
  searchInput.value?.focus()
}

onMounted(() => window.addEventListener('click', clickOutside))
onBeforeUnmount(() => window.removeEventListener('click', clickOutside))
</script>

<style scoped lang="scss">
.selector {
  position: relative;
  &-head {
    text-align: center;
    box-sizing: border-box;
  }
}

.selector-option {
  position: absolute;
  z-index: 1400;
  overflow-y: auto;
  width: 200px;
  background: rgb(228, 240, 240);
  box-shadow: 0 8px 16px rgba(0, 0, 0, 0.15);
  line-height: 25px;
  user-select: none;
  padding: 8px 0;
}

.selector-head {
  padding: 2px 10px;
  width: 100%;
}

.selector-head,
.selector-option-item {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.selector-head:hover .long-label,
.selector-option-item:hover .long-label {
  display: block;
  width: fit-content;
  animation: slide-label 5s linear infinite alternate;
}

.selector-search {
  position: sticky;
  top: -8px;
  z-index: 2;
  padding: 8px;
  background: rgb(228, 240, 240);

  input {
    box-sizing: border-box;
    width: 100%;
    height: 30px;
    padding: 0 9px;
    border: 1px solid rgba(0, 0, 0, 0.15);
    outline: none;
    background: rgba(255, 255, 255, 0.4);
    font: 12px SourceHanSansCN-Bold;
    color: black;

    &:focus {
      border-color: black;
    }
  }
}

.selector-option-item {
  width: 200px;
  height: 34px;
  box-sizing: border-box;
  font: 13px SourceHanSansCN-Bold;
  background-image: linear-gradient(90deg, black, black);
  background-repeat: repeat-y;
  background-position: -200px 0;
  padding: 0 16px;
  line-height: 34px;
  transition: background-position 0.2s, color 0.2s;
  cursor: pointer;
  text-align: center;

  &:hover {
    background-position: 0 0;
    color: white;
  }

  &-selected {
    background-color: black;
    color: white;
  }
}

.selector-empty {
  height: 34px;
  line-height: 34px;
  text-align: center;
  font: 12px SourceHanSansCN-Bold;
  color: rgba(0, 0, 0, 0.45);
}

@keyframes slide-label {
  from { transform: translateX(0%); }
  to { transform: translateX(-60%); }
}

::-webkit-scrollbar-track { border-radius: 0; }
::-webkit-scrollbar { -webkit-appearance: none; width: 6px; height: 6px; }
::-webkit-scrollbar-thumb {
  cursor: pointer;
  border-radius: 0;
  background: rgba(0, 0, 0, 0.15);
}
</style>

<style lang="scss">
.selector-enter-active,
.selector-leave-active {
  transition: opacity .16s, transform .16s;
  transform-origin: top center;
}
.selector-enter-from,
.selector-leave-to {
  opacity: 0;
  transform: translateY(-4px) scaleY(.96);
}
</style>
