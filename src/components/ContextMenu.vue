<script setup>
import { computed } from 'vue'
import { addToNextLocal } from '../utils/player'
import { FAVORITES_ID } from '../utils/collectionModel.mjs'
import { noticeOpen } from '../utils/dialog'
import { useCollectionStore } from '../store/collectionStore'
import { useOtherStore } from '../store/otherStore'

const otherStore = useOtherStore()
const collectionStore = useCollectionStore()

const menuItems = computed(() => {
  const trackId = otherStore.selectedItem?.id
  const items = [
    { id: 'play', name: '播放' },
    { id: 'next', name: '下一首播放' },
    {
      id: 'favorite',
      name: trackId && collectionStore.isFavorite(trackId) ? '取消收藏' : '收藏',
    },
  ]
  for (const playlist of collectionStore.playlists) {
    items.push({ id: `add:${playlist.id}`, name: `加入 · ${playlist.name}`, playlistId: playlist.id })
  }
  if (otherStore.selectedCollectionId && otherStore.selectedCollectionId !== FAVORITES_ID) {
    items.push({ id: 'remove', name: '从此歌单移除' })
  }
  items.push({ id: 'folder', name: '打开本地文件夹' })
  return items
})

async function select(item) {
  const track = otherStore.selectedItem
  otherStore.contextMenuShow = false
  if (!track) return

  try {
    if (item.id === 'play') addToNextLocal(track, true)
    if (item.id === 'next') addToNextLocal(track, false)
    if (item.id === 'folder') await windowApi.openLocalFolder(track.dirPath)
    if (item.id === 'favorite') await collectionStore.toggleFavorite(track.id)
    if (item.playlistId) {
      const added = await collectionStore.addTrack(item.playlistId, track.id)
      noticeOpen(added ? '已加入歌单' : '歌曲已在歌单中', 2)
    }
    if (item.id === 'remove') {
      await collectionStore.removeTrack(otherStore.selectedCollectionId, track.id)
      noticeOpen('已从歌单移除', 2)
    }
  } catch (error) {
    console.error('[context menu]', error)
    noticeOpen('操作失败', 3)
  }
}
</script>

<template>
  <div id="menu" class="context-menu">
    <div class="menu-container" v-show="otherStore.contextMenuShow">
      <div class="menu-item">
        <button class="item" type="button" @click="select(item)" v-for="item in menuItems" :key="item.id">{{ item.name }}</button>
      </div>
      <div class="menu-style menu-style1">+</div><div class="menu-style menu-style2">+</div>
      <div class="menu-style menu-style3">+</div><div class="menu-style menu-style4">+</div>
      <div class="menu-style5">MENU</div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.context-menu { position: absolute; overflow: hidden; z-index: var(--z-popover); }
.menu-container { min-width: 150Px; max-width: 260Px; max-height: 70vh; padding: 18Px 0; position: relative; overflow-y: auto; background-color: #202020; transform: translateY(-100%); animation: menu-in .2s cubic-bezier(.3,.79,.55,.99) forwards; &::-webkit-scrollbar { width: 4Px; } &::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, .2); } }
@keyframes menu-in { to { transform: translateY(0); } }
.menu-item { display: flex; flex-direction: column; .item { padding: 10Px 18Px; width: 100%; border: 0; box-sizing: border-box; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; background: transparent; font: 13Px SourceHanSansCN-Bold; color: white; text-align: left; transition: .2s; z-index: 2; &:hover { cursor: pointer; background-color: rgb(53 53 53 / 70%); } &:active { transform: scale(.95); } } }
.menu-style { position: absolute; color: white; } .menu-style1 { top: 0; left: 3Px; } .menu-style2 { top: 0; right: 3Px; } .menu-style3 { bottom: 0; right: 3Px; } .menu-style4 { bottom: 0; left: 3Px; }
.menu-style5 { font: 35Px Gilroy-ExtraBold; color: #393939; position: absolute; top: 10Px; left: 50%; transform: translateX(-50%); z-index: 1; }
</style>
