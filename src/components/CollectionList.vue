<script setup>
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { FAVORITES_ID } from '../utils/collectionModel.mjs'
import { dialogOpen, noticeOpen } from '../utils/dialog'
import { useCollectionStore } from '../store/collectionStore'

const route = useRoute()
const router = useRouter()
const collectionStore = useCollectionStore()
const creating = ref(false)
const newPlaylistName = ref('')

const items = computed(() => [
  {
    id: FAVORITES_ID,
    name: '收藏的歌曲',
    count: collectionStore.favoriteTrackIds.length,
    reserved: true,
  },
  ...collectionStore.playlists.map((playlist) => ({
    ...playlist,
    count: playlist.trackIds.length,
    reserved: false,
  })),
])

function openPlaylist(item) {
  router.push({ name: 'collection', params: { id: item.id } })
}

async function create() {
  try {
    const playlist = await collectionStore.createPlaylist(newPlaylistName.value)
    newPlaylistName.value = ''
    creating.value = false
    openPlaylist(playlist)
  } catch (error) {
    noticeOpen(error instanceof Error ? error.message : '歌单创建失败', 3)
  }
}

function cancelCreate() {
  newPlaylistName.value = ''
  creating.value = false
}

function requestDelete(item) {
  dialogOpen('删除歌单', `确定删除“${item.name}”吗？歌曲文件不会被删除。`, (confirmed) => {
    if (!confirmed) return
    collectionStore.deletePlaylist(item.id).then(() => {
      if (route.name === 'collection' && route.params.id === item.id) router.push('/mymusic')
    }).catch((error) => {
      console.error('[collections.delete]', error)
      noticeOpen('歌单删除失败', 3)
    })
  })
}
</script>

<template>
  <div class="collection-list">
    <button v-if="!creating" class="create-trigger" type="button" @click="creating = true">＋ 新建歌单</button>
    <form v-else class="create-form" @submit.prevent="create">
      <input v-model="newPlaylistName" maxlength="80" aria-label="歌单名称" placeholder="输入歌单名称" autofocus>
      <button type="submit">确认</button>
      <button type="button" @click="cancelCreate">取消</button>
    </form>

    <div
      v-for="item in items"
      :key="item.id"
      class="collection-item"
      :class="{ 'collection-item-selected': route.name === 'collection' && route.params.id === item.id }"
    >
      <button class="collection-open" type="button" @click="openPlaylist(item)">
        <div class="item-icon">
          <svg v-if="item.reserved" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
          </svg>
          <svg v-else viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 14h30M12 26h30M12 38h20" fill="none" stroke="currentColor" stroke-width="4" />
            <circle cx="44" cy="43" r="9" fill="none" stroke="currentColor" stroke-width="4" />
            <path d="M53 43V23" fill="none" stroke="currentColor" stroke-width="4" />
          </svg>
        </div>
        <div class="item-text">
          <span class="item-name">{{ item.name }}</span>
          <span class="item-count">{{ item.count }}首</span>
        </div>
      </button>
      <button
        v-if="!item.reserved"
        class="item-delete"
        type="button"
        :aria-label="`删除歌单 ${item.name}`"
        title="删除歌单"
        @click.stop="requestDelete(item)"
      >×</button>
    </div>
  </div>
</template>

<style scoped lang="scss">
.collection-list {
  width: 100%;
  height: 100%;
  overflow: auto;

  &::-webkit-scrollbar { display: none; }

  .create-trigger,
  .create-form {
    width: calc(100% - 16Px);
    min-height: 34Px;
    margin: 4Px 8Px 8Px;
    border: 0;
    border-bottom: .5Px solid rgba(0, 0, 0, .2);
    background: transparent;
    color: black;
    font: 12Px SourceHanSansCN-Bold;
  }

  .create-trigger {
    text-align: left;
    cursor: pointer;
    &:hover { opacity: .65; }
  }

  .create-form {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 6Px;

    input,
    button {
      min-width: 0;
      border: 0;
      outline: 0;
      background: transparent;
      color: black;
      font: 11Px SourceHanSansCN-Bold;
    }
    input { width: 100%; }
    button { padding: 0; cursor: pointer; &:hover { opacity: .65; } }
  }

  .collection-item {
    height: 62Px;
    box-sizing: border-box;
    padding: 8Px;
    display: flex;
    align-items: center;
    position: relative;
    transition: background-color .15s;
    cursor: pointer;

    &:hover { background-color: rgba(0, 0, 0, .035); }
    &.collection-item-selected { background-color: rgba(0, 0, 0, .05); box-shadow: inset 0 0 0 .5Px black; }

    .collection-open {
      min-width: 0;
      height: 100%;
      padding: 0;
      border: 0;
      background: transparent;
      display: flex;
      flex: 1;
      align-items: center;
      text-align: left;
      cursor: pointer;
    }

    .item-icon {
      width: 38Px;
      height: 38Px;
      flex: 0 0 38Px;
      margin-right: 10Px;
      padding: 5Px;
      box-sizing: border-box;
      color: black;
      svg { width: 100%; height: 100%; }
    }

    .item-text {
      min-width: 0;
      display: flex;
      flex: 1;
      flex-direction: column;
      align-items: flex-start;

      .item-name {
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font: 14Px SourceHanSansCN-Bold;
        color: black;
      }
      .item-count { font: 10Px SourceHanSansCN-Bold; color: rgb(107, 107, 107); }
    }

    .item-delete {
      width: 28Px;
      height: 28Px;
      padding: 0;
      border: 0;
      background: transparent;
      color: black;
      font: 20Px Bender-Bold;
      opacity: 0;
      cursor: pointer;
    }
    &:hover .item-delete { opacity: .55; }
    .item-delete:hover { opacity: 1; }
  }
}
</style>
