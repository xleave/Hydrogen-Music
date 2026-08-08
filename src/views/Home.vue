<template>
  <div>
    <main>
      <div class="home-header">
        <div class="header-router">
          <router-link class="button-music" to="/mymusic"></router-link>
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
  position: relative;
  .header-router {
    a { font: 18px SourceHanSansCN-Bold; color: black; outline: none; }
  }
}
.home-content { padding: 0 45px; height: calc(100% + 1px); overflow: auto; &::-webkit-scrollbar { display: none; } }
</style>
