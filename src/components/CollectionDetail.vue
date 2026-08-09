<script setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { FAVORITES_ID } from '../utils/collectionModel.mjs'
import { useCollectionStore } from '../store/collectionStore'
import { useLocalStore } from '../store/localStore'
import TrackDetailLayout from './TrackDetailLayout.vue'
import TrackTable from './TrackTable.vue'

const route = useRoute()
const collectionStore = useCollectionStore()
const localStore = useLocalStore()

const playlistId = computed(() => String(route.params.id || ''))
const playlist = computed(() => {
  if (playlistId.value === FAVORITES_ID) {
    return {
      id: FAVORITES_ID,
      name: '收藏的歌曲',
      trackIds: collectionStore.favoriteTrackIds,
    }
  }
  return collectionStore.playlistById(playlistId.value)
})
const trackIds = computed(() => playlist.value?.trackIds || [])
const songs = computed(() => localStore.resolveTrackIds(trackIds.value))
const unavailableCount = computed(() => Math.max(0, trackIds.value.length - songs.value.length))
const countText = computed(() => unavailableCount.value
  ? `${songs.value.length} 首歌曲 · ${unavailableCount.value} 首暂不可用`
  : `${songs.value.length} 首歌曲`)
</script>

<template>
  <TrackDetailLayout
    :title="playlist?.name || '歌单不存在'"
    :count-text="countText"
  >
    <template #cover>
      <svg v-if="playlistId === FAVORITES_ID" class="favorite-icon" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
        <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
      </svg>
      <svg v-else class="playlist-icon" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
        <path d="M12 14h30M12 26h30M12 38h20" fill="none" stroke="currentColor" stroke-width="4" />
        <circle cx="44" cy="43" r="9" fill="none" stroke="currentColor" stroke-width="4" />
        <path d="M53 43V23" fill="none" stroke="currentColor" stroke-width="4" />
      </svg>
    </template>

    <TrackTable
      :key="route.fullPath"
      :songs="songs"
      list-type="collection"
      :source-playlist-id="playlistId"
    />
  </TrackDetailLayout>
</template>
