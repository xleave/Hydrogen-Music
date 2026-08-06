<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const menuOpen = ref(false)
</script>

<template>
  <div>
    <main>
      <div class="home-header">
        <div class="header-router">
          <router-link class="button-music" to="/mymusic">本地音乐</router-link>
          <div class="user">
            <div class="user-container">
              <div class="user-head" @click="menuOpen = !menuOpen">
                <svg t="1672136404205" class="icon" viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" width="200" height="200"><path d="M511.997 551.041c-218.044 0-399.92 168.61-441.722 392.645l883.45-.439C911.607 719.432 729.83 551.041 511.997 551.041zM266.597 305.64c0 135.532 109.868 245.401 245.403 245.401 135.53 0 245.403-109.87 245.403-245.4C757.403 170.105 647.53 60.235 512 60.235c-135.535 0-245.403 109.87-245.403 245.406z" fill="#2c2c2c"/></svg>
                <div class="img-mask"></div>
              </div>
              <transition name="app-option">
                <div class="app-option app-option-active" v-show="menuOpen">
                  <div class="option" @click="router.push('/settings'); menuOpen = false">设置</div>
                  <div class="option-style option-style1"></div>
                  <div class="option-style option-style2"></div>
                  <div class="option-style option-style3"></div>
                  <div class="option-style option-style4"></div>
                </div>
              </transition>
            </div>
          </div>
        </div>
      </div>
      <div class="home-content">
        <router-view v-slot="{ Component }">
          <keep-alive><component :is="Component" /></keep-alive>
        </router-view>
      </div>
    </main>
  </div>
</template>

<style scoped lang="scss">
main { height: 100%; }
.home-header {
  margin: 30px 0 20px;
  display: flex;
  justify-content: center;
  align-items: center;
  .header-router {
    position: relative;
    a { font: 18px SourceHanSansCN-Bold; color: black; outline: none; }
    .user {
      position: absolute;
      top: 50%;
      right: -35px;
      transform: translateY(-50%);
      z-index: 999;
      .user-container { width: 25px; height: 25px; position: relative; }
      .user-head {
        width: 100%; height: 100%; border: 1px solid rgb(0 0 0 / 60%); border-radius: 50%; overflow: hidden; position: relative;
        &:hover { cursor: pointer; }
        svg { width: 100%; height: 100%; margin-top: 2px; }
        .img-mask { width: 100%; height: 100%; background-color: rgb(0 0 0 / 30%); opacity: 0; position: absolute; top: 0; left: 0; transition: .15s; &:hover { opacity: 1; } }
      }
      .app-option {
        padding: 0; width: 100px; height: 0; background-image: url('../assets/img/halftone.png'); background-size: 120%; background-color: #141414; overflow: hidden; position: absolute; top: 35px; left: -32.5px;
        &-active { height: 48px; padding: 6px 0; }
        .option { padding: 8px 14px; font: 14px SourceHanSansCN-Bold; color: white; text-align: left; &:hover { cursor: pointer; background-color: rgb(53 53 53 / 70%); } }
        .option-style { width: 4px; height: 4px; background-color: white; position: absolute; }
        .option-style1 { top: 4px; left: 4px; } .option-style2 { top: 4px; right: 4px; } .option-style3 { bottom: 4px; right: 4px; } .option-style4 { bottom: 4px; left: 4px; }
      }
    }
  }
}
.home-content { padding: 0 45px; height: calc(100% + 1px); overflow: auto; &::-webkit-scrollbar { display: none; } }
</style>

<style lang="scss">
.app-option-enter-active, .app-option-leave-active { transition: .2s; }
.app-option-enter-from, .app-option-leave-to { height: 0 !important; padding: 0 !important; }
</style>
