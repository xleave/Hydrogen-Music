<script setup>
import { computed } from 'vue'
import { onBeforeRouteUpdate, useRoute } from 'vue-router'
import { useLocalStore } from '../store/localStore'
import { storeToRefs } from 'pinia'
import TrackDetailLayout from './TrackDetailLayout.vue'
import TrackTable from './TrackTable.vue'

const route = useRoute()
const localStore = useLocalStore()
const { updateLocalMusicDetail } = localStore
const { currentType, currentSelectedInfo, currentSelectedSongs, currentSelectedFilePicUrl } = storeToRefs(localStore)

const songs = computed(() => currentSelectedSongs.value || [])

onBeforeRouteUpdate((to, from, next) => {
  updateLocalMusicDetail(to.name, to.query, to.params.id)
  currentType.value = to.name
  next()
})
</script>

<template>
  <TrackDetailLayout
    :title="currentSelectedInfo?.name || '本地音乐'"
    :count-text="`${songs.length} 首歌曲`"
    :cover-shadow="Boolean(currentSelectedFilePicUrl && currentType !== 'localFiles')"
  >
    <template #cover>
      <img v-if="currentSelectedFilePicUrl && currentType !== 'localFiles'" :src="currentSelectedFilePicUrl">
      <svg v-else-if="currentType === 'localArtist'" class="icon artist-icon" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
        <circle cx="32" cy="22" r="12" fill="none" stroke="currentColor" stroke-width="4" />
        <path d="M10 58c2-13 10-20 22-20s20 7 22 20" fill="none" stroke="currentColor" stroke-width="4" />
      </svg>
      <svg v-else-if="currentType === 'localAlbum'" class="icon album-icon" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
        <rect x="7" y="11" width="50" height="42" fill="none" stroke="currentColor" stroke-width="4" />
        <circle cx="32" cy="32" r="10" fill="none" stroke="currentColor" stroke-width="4" />
        <circle cx="32" cy="32" r="2.5" />
      </svg>
      <svg v-else class="icon folder-icon" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
        <path d="M418.133333 298.666667l-42.666666-42.666667H213.333333v512h640V298.666667H418.133333zM896 298.666667v512H170.666667V213.333333h226.133333l42.666667 42.666667H896v42.666667z m-298.666667 341.333333h170.666667v42.666667h-170.666667v-42.666667z" />
      </svg>
    </template>

    <TrackTable
      :key="route.fullPath"
      :songs="songs"
      :list-type="route.name"
    />
  </TrackDetailLayout>
</template>
