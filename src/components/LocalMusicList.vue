<script setup>
  import FolderTreeNode from './FolderTreeNode.vue'
  import LocalMusicClassify from './LocalMusicClassify.vue'
  import { useLibraryStore } from '../store/libraryStore'
  import { storeToRefs } from 'pinia'

  const libraryStore = useLibraryStore()
  const { listType2 } = storeToRefs(libraryStore)
  const props = defineProps(['folderlist', 'classifylist', 'type'])
</script>

<template>
  <div class="local-music-list">
    <div class="list-container">
      <div class="list-folder" v-if="listType2 === 0">
        <FolderTreeNode
          v-for="item in props.folderlist"
          :key="item.id || item.dirPath"
          :item="item"
          :type="props.type"
        />
      </div>
      <div class="list-albums" v-else-if="props.type === 'local' && listType2 === 1">
        <LocalMusicClassify :classifyData="props.classifylist.albums"></LocalMusicClassify>
      </div>
      <div class="list-artists" v-else-if="props.type === 'local' && listType2 === 2">
        <LocalMusicClassify :classifyData="props.classifylist.artists"></LocalMusicClassify>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
  .local-music-list{
    width: 100%;
    height: 100%;
    .list-container{
      width: 100%;
      height: 100%;
      .list-folder{
        display: flex;
        flex-direction: column;
      }
      .list-albums, .list-artists{ height: 100%; }
    }
  }
</style>
