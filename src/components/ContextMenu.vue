<script setup>
import { addToNextLocal } from '../utils/player'
import { useOtherStore } from '../store/otherStore'

const otherStore = useOtherStore()

function select(id) {
  if (id === 8) addToNextLocal(otherStore.selectedItem, true)
  if (id === 9) addToNextLocal(otherStore.selectedItem, false)
  if (id === 10) windowApi.openLocalFolder(otherStore.selectedItem.dirPath)
  otherStore.contextMenuShow = false
}
</script>

<template>
  <div id="menu" class="context-menu">
    <div class="menu-container" v-show="otherStore.contextMenuShow">
      <div class="menu-item">
        <div class="item" @click="select(item.id)" v-for="item in otherStore.menuTree" :key="item.id">{{ item.name }}</div>
      </div>
      <div class="menu-style menu-style1">+</div><div class="menu-style menu-style2">+</div>
      <div class="menu-style menu-style3">+</div><div class="menu-style menu-style4">+</div>
      <div class="menu-style5">MENU</div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.context-menu { position: absolute; overflow: hidden; }
.menu-container { padding: 18Px 0; position: relative; background-color: #202020; transform: translateY(-100%); animation: menu-in .2s cubic-bezier(.3,.79,.55,.99) forwards; }
@keyframes menu-in { to { transform: translateY(0); } }
.menu-item { display: flex; flex-direction: column; .item { padding: 10Px 18Px; width: 100%; font: 13Px SourceHanSansCN-Bold; color: white; text-align: left; transition: .2s; z-index: 2; &:hover { cursor: pointer; background-color: rgb(53 53 53 / 70%); } &:active { transform: scale(.95); } } }
.menu-style { position: absolute; color: white; } .menu-style1 { top: 0; left: 3Px; } .menu-style2 { top: 0; right: 3Px; } .menu-style3 { bottom: 0; right: 3Px; } .menu-style4 { bottom: 0; left: 3Px; }
.menu-style5 { font: 35Px Gilroy-ExtraBold; color: #393939; position: absolute; top: 10Px; left: 50%; transform: translateX(-50%); z-index: 1; }
</style>
