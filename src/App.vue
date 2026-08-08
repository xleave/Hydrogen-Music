<script setup>
  import { computed, defineAsyncComponent } from 'vue'
  import { useRoute } from 'vue-router'
  import Home from './views/Home.vue'
  import Title from './components/Title.vue'
  import WindowControl from './components/WindowControl.vue'
  import MusicWidget from './components/MusicWidget.vue'
  import MusicPlayer from './views/MusicPlayer.vue'
  import ContextMenu from './components/ContextMenu.vue'
  import GlobalDialog from './components/GlobalDialog.vue'
  import GlobalNotice from './components/GlobalNotice.vue'

  import { usePlayerStore } from './store/playerStore'

  // Diagnostics must never be a first-frame dependency. The component is
  // loaded after the root app has mounted; when disabled it does not sample.
  const PerformanceMonitor = defineAsyncComponent(() => import('./components/PerformanceMonitor.vue'))
  const playerStore = usePlayerStore()
  const route = useRoute()
  const settingsOverlay = computed(() => route.name === 'settings')
</script>

<template>
  <div class="mainWindow" :class="{ 'settings-overlay': settingsOverlay }">
    <Transition name="home">
      <Home class="home" v-show="playerStore.widgetState || settingsOverlay"></Home>
    </Transition>
  </div>
  <div class="globalWidget">
    <Title class="widget-title"></Title>
  </div>
  <div class="dragBar" data-tauri-drag-region>
    <WindowControl class="window-control"></WindowControl>
  </div>
  <Transition name="widget">
    <div class="musicWidget" v-if="playerStore.hasPlaylist" v-show="playerStore.widgetState">
      <MusicWidget></MusicWidget>
    </div>
  </Transition>
  <Transition name="player">
    <div class="musicPlayer" v-if="playerStore.hasPlaylist" v-show="!playerStore.widgetState">
      <MusicPlayer></MusicPlayer>
    </div>
  </Transition>
  <div class="context-menu">
    <ContextMenu></ContextMenu>
  </div>
  <div class="globalDialog">
    <GlobalDialog></GlobalDialog>
  </div>
  <div class="globalNotice">
    <GlobalNotice></GlobalNotice>
  </div>
  <PerformanceMonitor />
</template>

<style lang="scss">
  #app{
    user-select: none;
    margin: 0;
    padding: 0;
    max-width: 100%;
    position: fixed;
    left: 0;
    right: 0;
    top: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
  }
  .mainWindow{
    width: 100%;
    height: 100%;
    background: linear-gradient(rgba(176, 209, 217, 0.9) -20%,rgba(176, 209, 217, 0.4) 50%,rgba(176, 209, 217, 0.9) 120%);
    opacity: 0;
    animation: mainWindows-starting 0.8s cubic-bezier(.14,.91,.58,1) forwards;
    @keyframes mainWindows-starting {
      0%{background-color: rgba(222, 235, 239, 1);opacity: 0;transform: scale(1.3);}
      100%{background-color: rgb(255, 255, 255);opacity: 1;transform: scale(1);}
    }
    &.settings-overlay{
      position: relative;
      z-index: var(--z-settings);
    }
    .home{
      height: calc(100% - 78Px);
    }
  }
  .globalWidget{
    display: flex;
    flex-direction: row;
    align-items: center;
    position: absolute;
    top: 22Px;
    left: 45Px;
    z-index: var(--z-chrome);
    .widget-title{
      &:hover{
        cursor: pointer;
      }
    }
  }
  .dragBar{
    width: 100%;
    height: 35Px;
    background: transparent;
    position: fixed;
    top: 0;
    z-index: var(--z-chrome);
    -webkit-app-region: drag;
    .window-control{
      position: fixed;
      top: 13Px;
      right: 15Px;
      -webkit-app-region: no-drag;
      z-index: var(--z-chrome);
    }
  }
  .musicWidget{
    width: 680Px;
    height: 65Px;
    position: fixed;
    left: 50%;
    bottom: 35Px;
    transform: translateX(-50%);
    box-shadow: 0 0 15Px 2Px rgba(189, 189, 189, 0.1);
    z-index: var(--z-player-widget);
  }
  .musicPlayer{
    width: 100%;  
    height: 100%;
    position: absolute;
    top: 0;
    left: 0;
    z-index: var(--z-player);
  }
  .context-menu{
    z-index: var(--z-popover);
  }
  .globalDialog{
    z-index: var(--z-modal);
  }
  .globalNotice{
    bottom: 120Px;
    position: fixed;
    z-index: var(--z-notice);
  }

  .home-enter-active,
  .home-leave-active {
    transition: 0.4s cubic-bezier(.14,.91,.58,1);
  }

  .home-enter-from,
  .home-leave-to {
    transform: scale(0.9);
    opacity: 0;
  }

  .widget-enter-active,
  .widget-leave-active {
    transition: transform 0.5s cubic-bezier(.14,.91,.58,1);
  }

  .widget-enter-from,
  .widget-leave-to {
    transform: translate(-50%, 105Px);
  }

  .player-enter-active,
  .player-leave-active {
    transition: 0.5s cubic-bezier(.14,.91,.58,1);
  }

  .player-enter-from,
  .player-leave-to {
    transform: translateY(100%);
  }
  .video-enter-active,
  .video-leave-active {
    transition: 0.1s;
  }

  .video-enter-from,
  .video-leave-to {
    transform: scale(0.8);
    opacity: 0;
  }
  .fade-enter-active {
    transition: 0.4s;
  }
  .fade-leave-active {
    transition: 0.3s;
  }

  .fade-enter-from,
  .fade-leave-to {
    opacity: 0;
  }
</style>
