<script setup>
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useLibraryStore } from '../store/libraryStore'
import { useLocalStore } from '../store/localStore'
import { scanMusic } from '../utils/locaMusic'

const router = useRouter()
const libraryStore = useLibraryStore()
const localStore = useLocalStore()
const { librarySection, listType2 } = storeToRefs(libraryStore)

function changeSection(section) {
  librarySection.value = section
  router.push('/mymusic')
}

function changeType(type) {
  listType2.value = type
  router.push('/mymusic')
}

function refreshLocal() {
  localStore.isRefreshLocalFile = true
  scanMusic({ type: 'local', refresh: true })
  router.push('/mymusic')
}
</script>

<template>
  <div class="library-type">
    <div class="type-one">
      <div class="type-option">
        <button class="option" type="button" :class="{'option-selected': librarySection === 'local'}" @click="changeSection('local')">本地管理</button>
        <button class="option" type="button" :class="{'option-selected': librarySection === 'collections'}" @click="changeSection('collections')">播放清单</button>
      </div>
      <div class="option-tracker"><div class="tracker-line"></div><div class="tracker" :class="{'tracker-collections': librarySection === 'collections'}"></div></div>
    </div>
    <div class="type-two" v-if="librarySection === 'local'">
      <div class="type-option">
        <button class="option" type="button" :class="{'option-selected': listType2 === 0}" @click="changeType(0)">全部</button>
        <button class="option" type="button" :class="{'option-selected': listType2 === 1}" @click="changeType(1)">专辑</button>
        <button class="option" type="button" :class="{'option-selected': listType2 === 2}" @click="changeType(2)">歌手</button>
      </div>
      <span class="refresh" @click="refreshLocal()" v-show="localStore.localFolderSettings.length">刷新</span>
    </div>
    <div class="type-two collection-summary" v-else>收藏与自建歌单</div>
  </div>
</template>

<style scoped lang="scss">
.library-type {
  height: 50Px;
  .type-option { padding-left: 5Px; display: flex; }
  .option { margin-right: 20Px; padding: 0; border: 0; outline: 0; background: transparent; font: 16Px SourceHanSansCN-Bold; color: rgb(78 78 78); white-space: nowrap; &:hover { cursor: pointer; } }
  .option-selected { color: black; }
  .option-tracker { width: 100%; height: 3Px; position: relative; .tracker-line { width: 100%; height: .1Px; background-color: #6f6f6f; position: absolute; top: 50%; } .tracker { width: 64Px; height: 3Px; background-color: black; position: absolute; left: 4Px; top: 50%; transform: translateY(-50%); } .tracker-collections { left: 88Px; } }
  .type-two { margin-top: 4Px; padding-left: 5Px; display: flex; justify-content: space-between; align-items: center; .option { margin-right: 10Px; font-size: 12Px; } .refresh { margin-right: 10Px; font: 12Px SourceHanSansCN-Bold; color: #4e4e4e; &:hover { cursor: pointer; color: black; } } }
  .collection-summary { justify-content: flex-start; font: 12Px SourceHanSansCN-Bold; color: #4e4e4e; }
}
</style>
