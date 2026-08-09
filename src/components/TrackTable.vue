<script setup>
import { computed, ref } from 'vue'
import { RecycleScroller } from 'vue-virtual-scroller'
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css'
import { useRouter } from 'vue-router'
import { songTime2, addLocalMusicTOList, setShuffledList } from '../utils/player'
import { noticeOpen } from '../utils/dialog'
import { useCollectionStore } from '../store/collectionStore'
import { useLocalStore } from '../store/localStore'
import { useOtherStore } from '../store/otherStore'
import { usePlayerStore } from '../store/playerStore'
import { storeToRefs } from 'pinia'

const props = defineProps({
  songs: { type: Array, default: () => [] },
  listType: { type: String, default: 'local' },
  sourcePlaylistId: { type: String, default: null },
})

const router = useRouter()
const playerStore = usePlayerStore()
const collectionStore = useCollectionStore()
const localStore = useLocalStore()
const otherStore = useOtherStore()
const { songId, playMode } = storeToRefs(playerStore)

const searchQuery = ref('')
const sortMode = ref('default')

const filteredData = computed(() => {
  return localStore.filterTracks(props.songs, searchQuery.value)
})

const sortedData = computed(() => {
  const list = filteredData.value
  if (sortMode.value === 'modified_desc') {
    return [...list].sort((a, b) => (b.common?.modifiedAt ?? 0) - (a.common?.modifiedAt ?? 0))
  }
  return list
})

function formatTrack(item) {
  const sampleRate = item.format?.sampleRate ? `${item.format.sampleRate / 1000}KHz` : '--KHz'
  const bits = item.format?.bitsPerSample ? `${item.format.bitsPerSample}Bits` : '--Bits'
  const bitrate = item.format?.bitrate ? `${Math.round(item.format.bitrate / 1000)}Kpbs` : '--Kpbs'
  return `${sampleRate}/${bits}/${bitrate}`
}

function displayAlbum(item) {
  const album = item.common?.album
  return album && album !== '其他' ? album : '—'
}

function play(id) {
  const visibleSongs = sortedData.value
  const visibleIndex = visibleSongs.findIndex((song) => song.id === id)
  if (visibleIndex < 0) return
  addLocalMusicTOList(props.listType || router.currentRoute.value.name, visibleSongs, id, visibleIndex)
  if (playMode.value === 3) setShuffledList()
}

function openMenu(event, item) {
  otherStore.contextMenuShow = true
  otherStore.selectedItem = item
  otherStore.selectedCollectionId = props.sourcePlaylistId

  requestAnimationFrame(() => {
    const menuRoot = document.getElementById('menu')
    const menu = menuRoot?.querySelector('.menu-container')
    if (!menuRoot || !menu) return

    const screenWidth = document.body.clientWidth
    const screenHeight = document.body.clientHeight
    const menuWidth = Math.max(140, menu.offsetWidth)
    const menuHeight = Math.min(menu.scrollHeight, screenHeight * .7)
    const left = Math.max(6, Math.min(event.clientX, screenWidth - menuWidth - 6))
    const top = Math.max(6, Math.min(event.clientY, screenHeight - menuHeight - 6))
    menuRoot.style.right = ''
    menuRoot.style.bottom = ''
    menuRoot.style.left = `${left}Px`
    menuRoot.style.top = `${top}Px`
  })
}

function toggleFavorite(item) {
  collectionStore.toggleFavorite(item.id).catch((error) => {
    console.error('[collections.favorite]', error)
    noticeOpen('收藏状态保存失败', 3)
  })
}
</script>

<template>
  <div class="track-table">
    <div class="sort-bar">
      <div class="sort-options">
        <span class="sort-option" :class="{ 'sort-active': sortMode === 'default' }" @click="sortMode = 'default'">默认</span>
        <span class="sort-sep">·</span>
        <span class="sort-option" :class="{ 'sort-active': sortMode === 'modified_desc' }" @click="sortMode = 'modified_desc'">最近修改</span>
        <span v-if="searchQuery" class="search-count">{{ sortedData.length }} / {{ songs.length }}</span>
      </div>
      <label class="list-search">
        <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
          <circle cx="10.7" cy="10.7" r="6.2" fill="none" stroke="currentColor" stroke-width="1.8" />
          <path d="m15.3 15.3 4.3 4.3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
        <input v-model="searchQuery" type="search" placeholder="搜索当前列表" autocomplete="off" spellcheck="false">
      </label>
    </div>

    <RecycleScroller class="virtual-list" :items="sortedData" :item-size="68" key-field="id" v-slot="{ item, index }">
      <div class="list-item" :class="{ 'list-item-playing': songId === item.id }" @dblclick="play(item.id)" @contextmenu="openMenu($event, item)">
        <div class="item-title">
          <div class="item-state">
            <svg v-show="songId === item.id" class="icon playing-icon" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
              <path d="M158 614h67v342h-67zM361 0h67v956h-67zM596 273h67v683h-67zM832 137h67v819h-67z" />
            </svg>
            <div class="item-num" v-show="songId !== item.id">{{ index + 1 }}</div>
          </div>
          <div class="item-info">
            <span class="item-name">{{ item.common.title || item.common.localTitle }}</span>
            <div class="item-format">
              <div class="file-type">{{ item.format.container }}</div>
              <span class="format">{{ formatTrack(item) }}</span>
            </div>
          </div>
        </div>
        <div class="item-other">
          <div class="item-author">
            <span class="item-singer" v-if="item.common.artists && item.common.artists[0] !== '其他'" v-for="(singer, singerIndex) in item.common.artists" :key="singer">
              {{ singer }}{{ singerIndex === item.common.artists.length - 1 ? '' : '/' }}
            </span>
          </div>
          <span class="item-album" :title="displayAlbum(item)">{{ displayAlbum(item) }}</span>
          <span class="item-time">{{ songTime2(item.format.duration) }}</span>
          <button
            class="item-favorite"
            type="button"
            :aria-label="collectionStore.isFavorite(item.id) ? '取消收藏' : '收藏歌曲'"
            :aria-pressed="collectionStore.isFavorite(item.id)"
            :title="collectionStore.isFavorite(item.id) ? '取消收藏' : '收藏歌曲'"
            @click.stop="toggleFavorite(item)"
            @dblclick.stop
          >
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
              <path
                d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"
                :fill="collectionStore.isFavorite(item.id) ? 'currentColor' : 'none'"
                :stroke="collectionStore.isFavorite(item.id) ? 'none' : 'currentColor'"
                stroke-width="1.7"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        </div>
      </div>
    </RecycleScroller>
  </div>
</template>

<style scoped lang="scss">
.track-table {
  width: 100%;
  height: 100%;
  overflow: auto;
  user-select: text;

  &::-webkit-scrollbar { width: 5px; height: 10px; background-color: transparent; }
  &::-webkit-scrollbar-thumb { background-color: transparent; }
  &::-webkit-scrollbar-track { display: none; }
  &:hover::-webkit-scrollbar-thumb { background-color: rgba(0, 0, 0, .04); }

  .sort-bar {
    min-height: 34Px;
    padding: 6Px 8Px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18Px;

    .sort-options {
      min-width: 0;
      display: flex;
      align-items: center;
    }

    .sort-option {
      font: 11Px SourceHanSansCN-Bold;
      color: rgba(0, 0, 0, .38);
      cursor: pointer;
      transition: .15s;
      padding: 2Px 4Px;
      &:hover { color: rgba(0, 0, 0, .65); }
      &.sort-active { color: black; }
    }

    .sort-sep { font-size: 11Px; color: rgba(0, 0, 0, .2); padding: 0 1Px; }
    .search-count {
      margin-left: 8Px;
      font: 10Px Bender-Bold;
      color: rgba(0, 0, 0, .38);
      white-space: nowrap;
    }

    .list-search {
      width: 210Px;
      height: 26Px;
      flex: 0 0 210Px;
      display: flex;
      align-items: center;
      border-bottom: 1Px solid rgba(0, 0, 0, .18);
      transition: border-color .18s, background-color .18s;

      &:focus-within {
        border-bottom-color: rgba(0, 0, 0, .7);
        background-color: rgba(255, 255, 255, .16);
      }

      svg {
        margin: 0 7Px 0 5Px;
        width: 14Px;
        height: 14Px;
        flex: 0 0 14Px;
        color: rgba(0, 0, 0, .52);
      }

      input {
        width: 100%;
        height: 100%;
        padding: 0;
        border: 0;
        outline: 0;
        background: transparent;
        appearance: none;
        font: 11Px SourceHanSansCN-Bold;
        color: black;

        &::placeholder { color: rgba(0, 0, 0, .34); }
        &::-webkit-search-cancel-button { opacity: .5; cursor: pointer; }
      }
    }
  }

  .virtual-list { height: calc(100% - 34Px); }

  .list-item {
    padding: 12Px 8Px;
    box-sizing: border-box;
    display: flex;
    justify-content: space-between;
    align-items: center;
    transition: .2s;
    &:hover { cursor: default; background-color: rgba(0, 0, 0, .045); }

    .item-title {
      width: 44%;
      display: flex;
      align-items: center;

      svg { width: 14Px; height: 14Px; }
      .item-state {
        width: 26Px;
        flex: 0 0 26Px;
        .item-num { font: 14Px Geometos; color: rgb(127, 127, 127); }
      }

      .item-info {
        margin-left: 14Px;
        width: calc(100% - 40Px);
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: flex-start;

        .item-name {
          font: 15Px SourceHanSansCN-Bold;
          color: black;
          max-width: 100%;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .item-format {
          display: flex;
          align-items: center;
          .file-type {
            margin-right: 6Px;
            padding: 0 2Px;
            border: .5Px solid rgba(249, 190, 46, 1);
            font: 8Px Bender-Bold;
            color: rgba(249, 190, 46, 1);
          }
          .format { font: 10Px Bender-Bold; color: black; }
        }
      }
    }

    .item-other {
      margin-left: 14Px;
      width: 52%;
      display: grid;
      grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr) 68Px 30Px;
      align-items: center;
      column-gap: 18Px;
      span { font: 14Px SourceHanSansCN-Bold; color: black; }

      .item-author,
      .item-album {
        min-width: 0;
        text-align: left;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .item-author {
        .item-singer { transition: .1s; &:hover { cursor: pointer; opacity: .6; } }
      }

      .item-album { color: rgba(0, 0, 0, .58); }
      .item-time { width: 68Px; text-align: center; white-space: nowrap; }

      .item-favorite {
        width: 30Px;
        height: 30Px;
        padding: 5Px;
        border: 0;
        outline: 0;
        background: transparent;
        color: black;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: opacity .15s;
        &:hover { opacity: .65; }
        svg { width: 17Px; height: 17Px; }
      }
    }
  }

  .list-item:last-child { margin-bottom: 10Px; }
  .list-item-playing { background-color: rgba(0, 0, 0, .045); }
}
</style>
