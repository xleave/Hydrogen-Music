<script setup>
  import { useRouter } from 'vue-router'

  const router = useRouter()

  function windowControl(option) {
    if (option === 1) windowApi.windowMin('window-min')
    else if (option === 2) windowApi.windowMax('window-max')
    else windowApi.windowClose('window-close')
}

</script>

<template>
  <div class="window-control">
    <div @click="router.push('/settings')" class="settings" style="-webkit-app-region: no-drag">
      <svg viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg" width="200" height="200"><path d="M511.997 551.041c-218.044 0-399.92 168.61-441.722 392.645l883.45-.439C911.607 719.432 729.83 551.041 511.997 551.041zM266.597 305.64c0 135.532 109.868 245.401 245.403 245.401 135.53 0 245.403-109.87 245.403-245.4C757.403 170.105 647.53 60.235 512 60.235c-135.535 0-245.403 109.87-245.403 245.406z"/></svg>
    </div>
    <div @click="windowControl(1)" class="minimize" style="-webkit-app-region: no-drag">
      <svg t="1668091020963" viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" p-id="1210" width="200" height="200"><path d="M65.23884 456.152041 958.760137 456.152041l0 111.695918L65.23884 567.847959 65.23884 456.152041z" p-id="1211"></path></svg>
    </div>
    <div @click="windowControl(2)" class="maximize" style="-webkit-app-region: no-drag">
      <svg t="1668091098382" viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" p-id="1187" width="200" height="200"><path d="M128.576377 895.420553 128.576377 128.578424l766.846222 0 0 766.842129L128.576377 895.420553zM799.567461 224.434585 224.432539 224.434585l0 575.134923 575.134923 0L799.567461 224.434585z" p-id="1188"></path></svg>
    </div>
    <div @click="windowControl(0)" class="close-window" style="-webkit-app-region: no-drag">
      <svg t="1668091127480" viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" p-id="1341" width="200" height="200"><path d="M956.171172 875.411847l-80.757279 80.757279L511.997953 592.757279 148.586107 956.170149 67.828828 875.411847l363.411847-363.411847L67.828828 148.58713l80.757279-80.757279 363.411847 363.411847L875.413893 67.829851l80.757279 80.757279L592.756255 512 956.171172 875.411847z" p-id="1342"></path></svg>
    </div>
  </div>
</template>

<style scoped lang="scss">
  .window-control{
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8Px;
    div{
      width: 34Px;
      height: 34Px;
      box-sizing: border-box;
      display: flex;
      align-items: center;
      justify-content: center;
      opacity: 0.5;
      transition: 0.3s;
      padding: 8Px;
      &:hover{
        opacity: 1;
        cursor: pointer;
      }
      svg{
        width: 18Px;
        height: 18Px;
      }
    }
    
  }
</style>
