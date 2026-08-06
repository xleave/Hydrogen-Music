<script setup>
  import { useRouter } from 'vue-router'
  import { usePlayerStore } from '../store/playerStore';
  import { storeToRefs } from 'pinia';
  const router = useRouter()
  const playerStore = usePlayerStore()
  const { widgetState, lyricShow } = storeToRefs(playerStore)

  const backHome = () => {
    if(widgetState.value) router.push('/')
    widgetState.value = true
    lyricShow.value = false
  }
  windowApi.hidePlayer(() => {
    if(!widgetState.value) backHome()
  })
</script>

<template>
  <div class="title-container">
    <div class="title-logo" @click="backHome()">Hydrogen</div>
  </div>
</template>

<style scoped lang="scss">
  .title-container{
    position: relative;
    .title-logo{
      font: 28Px Gilroy-ExtraBold;
      color: rgb(26, 26, 26);
    }
    .title-player{
      width: 0;
      height: 8vh;
      background-color: rgba(255, 255, 255, 0.2);
      box-shadow: 0 0 12px 2px rgba(0, 0, 0, 0.02);
      backdrop-filter: blur(4px);
      position: absolute;
      top: 0;
      left: 0;
      z-index: 999;
      transition: 0.5s cubic-bezier(.3,.79,.55,.99);
      visibility: hidden;
      overflow: hidden;
      transform: translateX(-21px);
      .player-content{
        height: 100%;
        padding: 4px 1.2vh;
        display: flex;
        flex-direction: row;
        align-items: center;
        transform: translateX(-4px);
        transition: 0.2s 1s cubic-bezier(.06,.52,.29,1);
        opacity: 0;
        .cover{
          margin-right: 8px;
          img{
            width: 5.8vh;
            vertical-align: bottom;
          }
        }
        .music-info{
          width: 100%;
          display: flex;
          flex-direction: column;
          align-items: flex-start;
          overflow: hidden;
          white-space: nowrap;
          .music-name{
            margin-bottom: 0.5vh;
            font: 1.8vh SourceHanSansCN-Bold;
            color: black;
          }
          .music-time{
            width: 100%;
            display: flex;
            flex-direction: row;
            align-items: center;
            .music-progress{
              margin-left: 1px;
              width: 100% !important;
              height: 0.6vh !important;
              box-shadow: 0 0 0 0.5Px black;
              transition: 0.2s;
            }
            .remaining-time{
              width: 8vh;
              font: 1.5vh Bender-Bold;
              color: black;
              line-height: 1.5vh;
            }
          }
        }
      }
      .player-content-in{
        opacity: 1;
        visibility: visible;
        transform: translateX(0px);
        transition: 0.8s 1.2s cubic-bezier(.06,.52,.29,1);
      }
    }
    .title-player-in{
      width: 32vh;
      visibility: visible;
      transition: 0.4s 0.8s cubic-bezier(.06,.52,.29,1);
    }
  }
  .fade-enter-active,
  .fade-leave-active {
    transition: 0.2s;
  }

  .fade-enter-from,
  .fade-leave-to {
    transform: scale(0.9);
    opacity: 0;
  }
</style>
