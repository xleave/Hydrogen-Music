<script setup>
import { computed, ref } from 'vue'
import { RecycleScroller } from 'vue-virtual-scroller'
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css'
import { onBeforeRouteUpdate, useRouter } from 'vue-router'
import { nanoid } from 'nanoid'
import { songTime2, addLocalMusicTOList, setShuffledList } from '../utils/player'
import { useLocalStore } from '../store/localStore'
import { usePlayerStore } from '../store/playerStore'
import { useOtherStore } from '../store/otherStore'
import { storeToRefs } from 'pinia'

const router = useRouter()
const localStore = useLocalStore()
const { updateLocalMusicDetail } = localStore
const { currentType, currentSelectedInfo, currentSelectedSongs, currentSelectedFilePicUrl } = storeToRefs(localStore)
const playerStore = usePlayerStore()
const { songId, playMode } = storeToRefs(playerStore)
const otherStore = useOtherStore()

onBeforeRouteUpdate((to, from, next) => {
  updateLocalMusicDetail(to.name, to.query, to.params.id)
  currentType.value = to.name
  next()
  const list = document.getElementById('local-list')
  if (list) list.scrollTop = 0
})

function routerChange(operation) {
  if (operation) router.forward()
  else router.back()
}

const getData = computed(() => {
  const songs = currentSelectedSongs.value || []
  songs.forEach((item) => {
    if (!item.nid) Object.assign(item, { nid: nanoid() })
  })
  return songs
})

const songCount = computed(() => currentSelectedSongs.value?.length || 0)
const sortMode = ref('default')
const sortedData = computed(() => {
  const list = getData.value
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

function play(id) {
  const sourceIndex = currentSelectedSongs.value.findIndex((song) => song.id === id)
  addLocalMusicTOList(router.currentRoute.value.name, currentSelectedSongs.value, id, sourceIndex)
  if (playMode.value === 3) setShuffledList()
}

function openMenu(event, item) {
  otherStore.contextMenuShow = true
  otherStore.selectedItem = item
  const { clientX, clientY } = event
  const menuList = document.getElementById('menu')
  if (!menuList) return

  const screenWidth = document.body.clientWidth
  const screenHeight = document.body.clientHeight
  menuList.style.right = null
  menuList.style.bottom = null
  menuList.style.left = `${screenWidth - clientX < 120 ? screenWidth - 140 : clientX}Px`
  menuList.style.top = `${screenHeight - clientY < 240 ? screenHeight - 240 : clientY}Px`
}
</script>

<template>
  <div class="local-music-detail">
    <div class="local-music-container">
      <div class="view-control">
        <svg @click="routerChange(0)" class="router-last" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
          <path d="M716.608 1010.112L218.88 512.384 717.376 13.888l45.248 45.248-453.248 453.248 452.48 452.48z" />
        </svg>
        <svg @click="routerChange(1)" class="router-next" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
          <path d="M264.896 1010.112l497.728-497.728L264.128 13.888 218.88 59.136l453.248 453.248-452.48 452.48z" />
        </svg>
      </div>

      <div class="local-music-header">
        <div class="local-music-cover" :class="{ 'cover-shadow': currentSelectedFilePicUrl && currentType !== 'localFiles' }">
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
        </div>
        <div class="local-music-summary">
          <span class="local-music-title">{{ currentSelectedInfo?.name || '本地音乐' }}</span>
          <span class="local-music-count">{{ songCount }} 首歌曲</span>
        </div>
      </div>

      <div id="local-list" class="local-music-list">
        <div class="sort-bar">
          <span class="sort-option" :class="{ 'sort-active': sortMode === 'default' }" @click="sortMode = 'default'">默认</span>
          <span class="sort-sep">·</span>
          <span class="sort-option" :class="{ 'sort-active': sortMode === 'modified_desc' }" @click="sortMode = 'modified_desc'">最近修改</span>
        </div>

        <RecycleScroller class="virtual-list" :items="sortedData" :item-size="68" key-field="nid" v-slot="{ item, index }">
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
              <span class="item-time">{{ songTime2(item.format.duration) }}</span>
            </div>
          </div>
        </RecycleScroller>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.local-music-detail {
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;

  .local-music-container {
    width: 100%;
    height: 100%;

    .view-control {
      margin-left: -8Px;
      height: 32Px;
      svg {
        padding: 8Px;
        width: 32Px;
        height: 32Px;
        float: left;
        transition: .2s;
        &:hover { cursor: pointer; opacity: .7; }
        &:active { transform: scale(.9); }
      }
      .router-last { margin-right: 20Px; }
    }

    .local-music-header {
      height: 82Px;
      box-sizing: border-box;
      padding: 10Px 18Px 12Px 28Px;
      display: flex;
      align-items: center;
      border-bottom: .5Px solid rgba(0, 0, 0, .1);
      user-select: text;

      .local-music-cover {
        flex: 0 0 56Px;
        width: 56Px;
        height: 56Px;
        display: flex;
        align-items: center;
        justify-content: center;
        color: black;
        overflow: hidden;

        img,
        svg { width: 100%; height: 100%; object-fit: cover; }
        .folder-icon { width: 52Px; height: 52Px; }
        .artist-icon,
        .album-icon { width: 50Px; height: 50Px; }
      }

      .cover-shadow {
        border: .5Px solid rgb(218, 218, 218);
        box-shadow: 0 0 6Px 1Px rgba(0, 0, 0, .03);
      }

      .local-music-summary {
        min-width: 0;
        margin-left: 16Px;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        justify-content: center;

        .local-music-title {
          max-width: 58vw;
          font: 20Px SourceHanSansCN-Bold;
          color: black;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .local-music-count {
          margin-top: 3Px;
          font: 11Px SourceHanSansCN-Bold;
          color: rgba(0, 0, 0, .48);
        }
      }
    }

    .local-music-list {
      width: 100%;
      height: calc(100% - 114Px);
      overflow: auto;
      user-select: text;

      &::-webkit-scrollbar { width: 5px; height: 10px; background-color: transparent; }
      &::-webkit-scrollbar-thumb { background-color: transparent; }
      &::-webkit-scrollbar-track { display: none; }
      &:hover::-webkit-scrollbar-thumb { background-color: rgba(0, 0, 0, .04); }

      .sort-bar {
        padding: 8Px 8Px 6Px 8Px;
        display: flex;
        align-items: center;
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
          width: 50%;
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
          width: 45%;
          display: flex;
          justify-content: space-between;
          span { font: 14Px SourceHanSansCN-Bold; color: black; }
          .item-author {
            width: 70%;
            text-align: left;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            .item-singer { transition: .1s; &:hover { cursor: pointer; opacity: .6; } }
          }
          .item-time { width: 30%; }
        }
      }

      .list-item:last-child { margin-bottom: 10Px; }
      .list-item-playing { background-color: rgba(0, 0, 0, .045); }
    }
  }
}
</style>
