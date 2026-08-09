<script setup>
import { useRouter } from 'vue-router'

defineProps({
  title: { type: String, default: '本地音乐' },
  countText: { type: String, default: '0 首歌曲' },
  coverShadow: { type: Boolean, default: false },
})

const router = useRouter()

function routerChange(forward) {
  if (forward) router.forward()
  else router.back()
}
</script>

<template>
  <div class="local-music-detail">
    <div class="local-music-container">
      <div class="view-control">
        <svg @click="routerChange(false)" class="router-last" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
          <path d="M716.608 1010.112L218.88 512.384 717.376 13.888l45.248 45.248-453.248 453.248 452.48 452.48z" />
        </svg>
        <svg @click="routerChange(true)" class="router-next" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
          <path d="M264.896 1010.112l497.728-497.728L264.128 13.888 218.88 59.136l453.248 453.248-452.48 452.48z" />
        </svg>
      </div>

      <div class="local-music-header">
        <div class="local-music-cover" :class="{ 'cover-shadow': coverShadow }">
          <slot name="cover" />
        </div>
        <div class="local-music-summary">
          <span class="local-music-title">{{ title }}</span>
          <span class="local-music-count">{{ countText }}</span>
        </div>
      </div>

      <div class="detail-content">
        <slot />
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

        :deep(img),
        :deep(svg) { width: 100%; height: 100%; object-fit: cover; }
        :deep(.folder-icon) { width: 52Px; height: 52Px; }
        :deep(.artist-icon),
        :deep(.album-icon),
        :deep(.playlist-icon),
        :deep(.favorite-icon) { width: 50Px; height: 50Px; }
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

    .detail-content {
      width: 100%;
      height: calc(100% - 114Px);
    }
  }
}
</style>
