<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLibraryStore } from '../store/libraryStore'
import { useLocalStore } from '../store/localStore'

const props = defineProps({
  item: { type: Object, required: true },
  type: { type: String, required: true },
  depth: { type: Number, default: 0 },
})

const route = useRoute()
const router = useRouter()
const libraryStore = useLibraryStore()
const localStore = useLocalStore()

const id = computed(() => props.item.id || props.item.dirPath)
const hasChildren = computed(() => Array.isArray(props.item.children) && props.item.children.length > 0)
const expanded = computed(() => hasChildren.value && libraryStore.folderExpanded(id.value))
const selected = computed(() => route.query.id === id.value)

function toggleChildren() {
  if (hasChildren.value) libraryStore.toggleFolder(id.value)
}

function showFiles() {
  localStore.currentSelectedFile = props.item
  router.push({
    name: 'localFiles',
    query: { id: id.value, type: props.type },
  })
}
</script>

<template>
  <div
    class="folder-tree-node"
    :class="{
      'folder-tree-node-root': depth === 0,
      'folder-tree-node-child': depth > 0,
      'list-item-open': expanded,
      'list-item-selected': selected,
    }"
    @click.stop="showFiles"
  >
    <div class="folder">
      <div class="folder-img">
        <svg class="icon" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
          <path d="M418.133333 298.666667l-42.666666-42.666667H213.333333v512h640V298.666667H418.133333zM896 298.666667v512H170.666667V213.333333h226.133333l42.666667 42.666667H896v42.666667z m-298.666667 341.333333h170.666667v42.666667h-170.666667v-42.666667z" fill="#000000" />
        </svg>
      </div>
      <div class="folder-name">
        <span class="name">{{ item.name }}</span>
      </div>
      <div
        v-if="hasChildren"
        class="folder-more"
        :class="{ 'folder-more-open': expanded }"
        @click.stop="toggleChildren"
      >
        <svg class="icon" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
          <path d="M533.333333 605.866667L341.333333 413.866667l29.866667-29.866667 162.133333 162.133333L695.466667 384l29.866666 29.866667-192 192z" fill="#000000" />
        </svg>
      </div>
    </div>

    <Transition name="children">
      <div v-if="expanded" class="children-folder">
        <FolderTreeNode
          v-for="child in item.children"
          :key="child.id || child.dirPath"
          :item="child"
          :type="type"
          :depth="depth + 1"
        />
      </div>
    </Transition>
  </div>
</template>

<style scoped lang="scss">
.folder-tree-node {
  width: 100%;
  padding: 8Px;

  &:hover {
    cursor: pointer;
    background-color: rgba(0, 0, 0, .02);
  }

  &.list-item-open { background-color: rgba(0, 0, 0, .02); }
  &.list-item-selected {
    background-color: rgba(0, 0, 0, .05) !important;
    box-shadow: inset 0 0 0 .5Px black;
  }

  .folder {
    width: 100%;
    display: flex;
    flex-direction: row;
    align-items: center;
  }

  .folder-img {
    margin-right: 10Px;
    flex: 0 0 auto;
    svg { width: 100%; height: 100%; }
  }

  .folder-name {
    min-width: 0;
    .name {
      color: black;
      text-align: left;
      overflow: hidden;
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 1;
      word-break: break-all;
    }
  }

  .folder-more {
    flex: 0 0 auto;
    transition: transform .2s, opacity .2s;
    opacity: .6;
    &:hover { opacity: 1; }
    svg { width: 100%; height: 100%; }
  }

  .folder-more-open { transform: rotate(180deg); }

  .children-enter-active,
  .children-leave-active { transition: .1s; }
  .children-enter-from,
  .children-leave-to { transform: scale(.95); opacity: 0; }
}

.folder-tree-node-root {
  overflow: hidden;
  .folder-img { width: 30Px; height: 30Px; }
  .folder-name {
    width: calc(100% - 75Px);
    .name { font: 14Px SourceHanSansCN-Bold; }
  }
  .folder-more { width: 35Px; height: 35Px; }
}

.folder-tree-node-child {
  transition: background-color .1s;
  .folder-img { width: 25Px; height: 25Px; }
  .folder-name {
    width: calc(100% - 65Px);
    .name { font: 13Px SourceHanSansCN-Bold; }
  }
  .folder-more { width: 30Px; height: 30Px; }
}
</style>
