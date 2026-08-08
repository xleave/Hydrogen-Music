<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { changeProgress } from '../utils/player'
import { usePlayerStore } from '../store/playerStore'

const playerStore = usePlayerStore()
const {
  currentIndex,
  currentMusic,
  isLyricDelay,
  lyric,
  lyricAnimationRevision,
  lyricBlur,
  lyricInterludeTime,
  lyricShow,
  lyricSize,
  lyricType,
  lyricsObjArr,
  playing,
  playerChangeSong,
  rlyricSize,
  songList,
  tlyricSize,
  widgetState,
} = storeToRefs(playerStore)

const activeIndex = ref(-1)
const interludeIndex = ref(null)
const interludeAnimation = ref(false)
const interludeRemainingTime = ref(0)
const isLyricActive = ref(true)
const lyricTrack = ref(null)
let activeTimer = null
let scrollTimer = null
let interludeTimer = null
let wheelFrame = null
let returnTimer = null
let manualScrollOffset = 0
let pendingWheelDelta = 0
let wheelVelocity = 0
let isReturning = false

const timestampPattern = /\[(\d{2}):(\d{2})(?:\.|:)(\d{2,3})\]/

function parseTimestamp(value) {
  const match = value.match(timestampPattern)
  if (!match) return null
  const milliseconds = Number(match[3].padEnd(3, '0'))
  return Number(match[1]) * 60 + Number(match[2]) + milliseconds / 1000
}

function timedTextMap(lines) {
  const result = new Map()
  for (const line of lines || []) {
    const time = parseTimestamp(line)
    if (time === null) continue
    result.set(time.toFixed(3), line.replace(timestampPattern, '').trim())
  }
  return result
}

function updateUnavailableTypes(original, translated, romanized) {
  const entries = [
    ['noOriginal', original],
    ['noTrans', translated],
    ['noRoma', romanized],
  ]
  for (const [name, available] of entries) {
    const index = lyricType.value.indexOf(name)
    if (available && index >= 0) lyricType.value.splice(index, 1)
    if (!available && index < 0) lyricType.value.push(name)
  }
}

function parseLyrics(value) {
  const originalLines = value.lrc.lyric.split(/\r?\n/)
  const translated = value.tlyric?.lyric?.split(/\r?\n/) || null
  const romanized = value.romalrc?.lyric?.split(/\r?\n/) || null
  updateUnavailableTypes(originalLines, translated, romanized)

  const translatedByTime = timedTextMap(translated)
  const romanizedByTime = timedTextMap(romanized)
  const parsed = []
  for (const line of originalLines) {
    const time = parseTimestamp(line)
    if (time === null) continue
    const text = line.replace(timestampPattern, '').trim()
    if (!text) continue
    if (text.includes('纯音乐')) {
      return [{ lyric: '纯音乐，请欣赏', time: 0 }]
    }
    const key = time.toFixed(3)
    parsed.push({
      lyric: text,
      time,
      tlyric: translatedByTime.get(key),
      rlyric: romanizedByTime.get(key),
    })
  }
  if (parsed.length) return parsed.sort((left, right) => left.time - right.time)

  updateUnavailableTypes(originalLines, null, null)
  return originalLines
    .map((line) => line.trim())
    .filter(Boolean)
    .map((text) => ({ active: true, lyric: text, time: 0 }))
}

const displayedLyrics = computed(() => {
  if (lyric.value && !lyricsObjArr.value) {
    lyricsObjArr.value = parseLyrics(lyric.value)
    lyric.value = null
    activeIndex.value = -1
    interludeAnimation.value = false
    resetManualScroll(true)
    if (!lyricShow.value && !widgetState.value) {
      lyricShow.value = true
      playerChangeSong.value = false
    }
  }
  if (!lyricsObjArr.value) updateUnavailableTypes(null, null, null)
  return lyricsObjArr.value || []
})

const lineHeight = computed(() => {
  const size = [
    lyricType.value.includes('original') && !lyricType.value.includes('noOriginal') ? Number(lyricSize.value) : 0,
    lyricType.value.includes('trans') && !lyricType.value.includes('noTrans') ? Number(tlyricSize.value) : 0,
    lyricType.value.includes('roma') && !lyricType.value.includes('noRoma') ? Number(rlyricSize.value) : 0,
  ].reduce((sum, value) => sum + value, 0)
  return size * 1.5 + 30
})

const scrollAreaHeight = computed(() => displayedLyrics.value.length * lineHeight.value)
const lineOffset = computed(() => {
  const base = -(scrollAreaHeight.value - 260)
  return base - (activeIndex.value + 1) * lineHeight.value
})

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value))
}

function setTrackOffset(offset) {
  manualScrollOffset = clamp(offset, -scrollAreaHeight.value, scrollAreaHeight.value)
  if (lyricTrack.value) {
    lyricTrack.value.style.transform = `translate3d(0, ${manualScrollOffset}px, 0)`
  }
}

function currentTrackOffset(track) {
  const transform = getComputedStyle(track).transform
  if (!transform || transform === 'none') return manualScrollOffset
  try {
    return new DOMMatrixReadOnly(transform).m42
  } catch {
    const match = transform.match(/^matrix\([^,]+,[^,]+,[^,]+,[^,]+,[^,]+,\s*([^)]+)\)$/)
    return match ? Number(match[1]) || manualScrollOffset : manualScrollOffset
  }
}

function cancelWheelFrame() {
  if (wheelFrame !== null) cancelAnimationFrame(wheelFrame)
  wheelFrame = null
  pendingWheelDelta = 0
  wheelVelocity = 0
}

function cancelReturnAnimation(preserveVisualPosition = false) {
  if (returnTimer !== null) clearTimeout(returnTimer)
  returnTimer = null
  const track = lyricTrack.value
  const visualOffset = preserveVisualPosition && isReturning && track
    ? currentTrackOffset(track)
    : null
  isReturning = false
  track?.classList.remove('lyric-track-returning')
  if (visualOffset !== null && track) {
    track.style.transition = 'none'
    setTrackOffset(visualOffset)
    // Commit the current visual transform before wheel inertia resumes.
    void track.offsetHeight
    track.style.transition = ''
  }
}

function resetManualScroll(immediate = false) {
  clearTimeout(scrollTimer)
  scrollTimer = null
  cancelWheelFrame()
  cancelReturnAnimation()
  setTrackOffset(0)
  isLyricActive.value = true
  if (immediate && lyricTrack.value) lyricTrack.value.style.transition = 'none'
  requestAnimationFrame(() => {
    if (lyricTrack.value) lyricTrack.value.style.transition = ''
  })
}

function findActiveLine(seek) {
  const lines = displayedLyrics.value
  let low = 0
  let high = lines.length - 1
  let result = -1
  while (low <= high) {
    const middle = Math.floor((low + high) / 2)
    if (lines[middle].time <= seek + 0.2) {
      result = middle
      low = middle + 1
    } else {
      high = middle - 1
    }
  }
  return result
}

function updateActiveLine() {
  const seek = currentMusic.value?.seek()
  if (typeof seek !== 'number') return
  const nextIndex = findActiveLine(seek)
  if (nextIndex !== activeIndex.value) {
    const previousIndex = activeIndex.value
    if (!isLyricActive.value && !isReturning && previousIndex >= -1) {
      setTrackOffset(manualScrollOffset + (nextIndex - previousIndex) * lineHeight.value)
    }
    activeIndex.value = nextIndex
  }

  const nextLine = displayedLyrics.value[nextIndex + 1]
  const remaining = nextLine ? nextLine.time - seek : 0
  interludeRemainingTime.value = Math.max(0, Math.trunc(remaining - 1))
  if (remaining >= lyricInterludeTime.value) {
    interludeIndex.value = nextIndex
    if (!interludeAnimation.value && !interludeTimer) {
      interludeTimer = setTimeout(() => {
        interludeAnimation.value = true
        interludeTimer = null
      }, 1000)
    }
  } else {
    clearTimeout(interludeTimer)
    interludeTimer = null
    interludeAnimation.value = false
    interludeIndex.value = null
  }
}

function startActiveTimer() {
  clearInterval(activeTimer)
  activeTimer = setInterval(updateActiveLine, 200)
  updateActiveLine()
}

function lineStyle(index) {
  const distance = Math.abs(index - activeIndex.value)
  return {
    transform: `translate3d(0, ${lineOffset.value}px, 0)`,
    transitionDelay: isLyricDelay.value && index >= activeIndex.value
      ? `${Math.min((index - activeIndex.value) * 0.05, 0.5)}s`
      : '0s',
    '--lyric-blur': lyricBlur.value && isLyricActive.value
      ? `${Math.min(distance * 0.25, 1.8)}px`
      : '0px',
  }
}

function changeProgressLyric(time, index) {
  resetManualScroll(true)
  activeIndex.value = index
  changeProgress(time)
}

function normalizeWheelDelta(event) {
  let delta = event.deltaY
  if (event.deltaMode === WheelEvent.DOM_DELTA_LINE) delta *= 18
  else if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE) delta *= 180
  return clamp(delta, -180, 180)
}

function runWheelFrame() {
  wheelFrame = null
  if (pendingWheelDelta !== 0) {
    wheelVelocity += pendingWheelDelta * 0.24
    pendingWheelDelta = 0
  }
  wheelVelocity = clamp(wheelVelocity, -72, 72)
  if (Math.abs(wheelVelocity) < 0.08) {
    wheelVelocity = 0
    return
  }

  setTrackOffset(manualScrollOffset - wheelVelocity)
  wheelVelocity *= 0.82
  wheelFrame = requestAnimationFrame(runWheelFrame)
}

function returnToActiveLine() {
  cancelWheelFrame()
  cancelReturnAnimation()
  const track = lyricTrack.value
  if (!track) {
    manualScrollOffset = 0
    isLyricActive.value = true
    return
  }

  isReturning = true
  track.classList.add('lyric-track-returning')
  requestAnimationFrame(() => setTrackOffset(0))
  returnTimer = setTimeout(() => {
    track.classList.remove('lyric-track-returning')
    returnTimer = null
    isReturning = false
    isLyricActive.value = true
  }, 620)
}

function handleWheel(event) {
  if (isReturning) cancelReturnAnimation(true)
  if (isLyricActive.value) isLyricActive.value = false

  pendingWheelDelta = clamp(pendingWheelDelta + normalizeWheelDelta(event), -360, 360)
  if (wheelFrame === null) wheelFrame = requestAnimationFrame(runWheelFrame)

  clearTimeout(scrollTimer)
  scrollTimer = setTimeout(returnToActiveLine, 3000)
}

watch(
  () => [widgetState.value, playing.value, lyricShow.value, lyricsObjArr.value],
  () => {
    if (!widgetState.value && playing.value && lyricShow.value && lyricsObjArr.value) startActiveTimer()
    else clearInterval(activeTimer)
  },
)

watch(lyricAnimationRevision, () => {
  resetManualScroll(true)
})

watch([lyricSize, tlyricSize, rlyricSize, lyricType], () => {
  resetManualScroll(true)
}, { deep: true })

onBeforeUnmount(() => {
  clearInterval(activeTimer)
  clearTimeout(scrollTimer)
  clearTimeout(interludeTimer)
  clearTimeout(returnTimer)
  cancelWheelFrame()
})
</script>

<template>
  <div class="lyric-container">
    <Transition name="fade">
      <div v-show="lyricsObjArr && lyricShow && lyricType.includes('original')" class="lyric-area" @wheel.prevent="handleWheel">
        <div class="lyric-scroll-area" :style="{ height: `${scrollAreaHeight}px` }"></div>
        <div ref="lyricTrack" class="lyric-track">
          <div class="lyric-line" :style="lineStyle(index)" v-for="(item, index) in displayedLyrics" :key="`${item.time}-${index}`" v-show="item.lyric">
            <div class="line" @click="changeProgressLyric(item.time, index)" :class="{'line-highlight': index === activeIndex, 'lyric-inactive': !isLyricActive || item.active}">
              <span class="roma" :style="{'font-size': `${rlyricSize}px`}" v-if="item.rlyric && lyricType.includes('roma')">{{item.rlyric}}</span>
              <span class="original" :style="{'font-size': `${lyricSize}px`}" v-if="lyricType.includes('original')">{{item.lyric}}</span>
              <span class="trans" :style="{'font-size': `${tlyricSize}px`}" v-if="item.tlyric && lyricType.includes('trans')">{{item.tlyric}}</span>
              <div class="hilight" :class="{'hilight-active': index === activeIndex}"></div>
            </div>
            <div v-if="activeIndex !== -1 && interludeIndex === index" class="music-interlude" :class="{'music-interlude-in': interludeAnimation}">
              <div class="interlude-left">
                <div class="diamond">
                  <div class="diamond-inner"></div>
                </div>
              </div>
              <div class="interlude-right">
                <div class="triangle"></div>
                <span class="remaining">THE REMAINING TIME: {{interludeRemainingTime}}</span>
                <div class="interlude-title">
                  <span class="title">MUSIC INTERLUDE</span>
                  <div class="title-style">
                    <svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="49" height="50" viewBox="0 0 49 50" fill="none"><defs><rect id="path_0" x="0" y="0" width="49" height="50"/></defs><g opacity="1" transform="translate(0 0) rotate(0 24.5 25)"><mask id="bg-mask-0" fill="white"><use xlink:href="#path_0"/></mask><g mask="url(#bg-mask-0)"><path style="stroke:#FFFFFF;stroke-width:1" transform="translate(46 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:1" transform="translate(27 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:1" transform="translate(48 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:2" transform="translate(19 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:1" transform="translate(34 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:1" transform="translate(16 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:1" transform="translate(43 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:2" transform="translate(23 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:2" transform="translate(12 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:1" transform="translate(5 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:2" transform="translate(8 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:2" transform="translate(30 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:3" transform="translate(1 0)" d="M0,0L0,100"/><path style="stroke:#FFFFFF;stroke-width:3" transform="translate(40 0)" d="M0,0L0,100"/></g></g></svg>
                  </div>
                </div>
                <div class="interlude-progress"></div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
    <Transition name="fade">
      <div v-show="!lyricsObjArr || !lyricType.includes('original')" class="lyric-nodata">
          <div class="line1"></div>
          <span class="tip">Lyric-Area</span>
          <div class="line2"></div>
      </div>
    </Transition>

    <span class="song-quality" v-if="songList?.[currentIndex]">{{songList[currentIndex].sampleRate}}KHz/{{songList[currentIndex].bitsPerSample}}Bits/{{songList[currentIndex].bitrate}}Kbps</span>
    <div class="border border1"></div>
    <div class="border border2"></div>
    <div class="border border3"></div>
    <div class="border border4"></div>
  </div>
</template>

<style scoped lang="scss">
  .lyric-container{
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1;
    .lyric-area{
      width: calc(100% - 3vh);
      height: calc(100% - 3vh);
      overflow: hidden;
      transition: 0.3s cubic-bezier(.30,0,.12,1);
      .lyric-scroll-area{
        width: 100%;
        transition: 0.3s;
      }
      .lyric-track{
        transform: translate3d(0, 0, 0);
        will-change: transform;
      }
      .lyric-track-returning{
        transition: transform 0.58s cubic-bezier(.4,0,.12,1);
      }
      .lyric-line{
        margin-bottom: 10Px;
        width: 100%;
        text-align: left;
        transition: 0.58s cubic-bezier(.4,0,.12,1);
        .line{
          padding: 10Px 130Px 10Px 25Px;
          width: 100%;
          height: 100%;
          position: relative;
          overflow: hidden;
          display: flex;
          flex-direction: column;
          align-items: flex-start;
          transition: 0.4s cubic-bezier(.30,0,.12,1);
          user-select: text;
          &:hover{
            cursor: pointer;
            background-color: rgba(0, 0, 0, 0.045);
          }
          &:active{
            transform: scale(0.9);
            filter: blur(0) !important;
          }
          .original, .trans, .roma{
            font: 20Px SourceHanSansCN-Bold;
            font-weight: bold;
            color: black;
            text-align: left;
            display: inline-block;
            transition: 0.5s cubic-bezier(.30,0,.12,1);
            filter: blur(var(--lyric-blur));
          }
          .hilight{
            width: 100%;
            height: 100%;
            background-color: black;
            position: absolute;
            z-index: -1;
            top: 0;
            left: 0;
            transform: translateX(-101%);
            transition: 0.55s cubic-bezier(.30,0,.12,1);
          }
          .hilight-active{
            transform: translateX(0);
            transition: 0.62s cubic-bezier(.30,0,.12,1);
          }
        }
        .lyric-inactive{
          filter: blur(0) !important;
          span{
            transform: scale(1.05);
          }
        }
        .line-highlight{
          transition-duration: 0.4s;
          .original, .trans, .roma{
            transform-origin: left center;
            transform: scale(1.15) translateX(26px);
            color: white;
            transition: 0.4s cubic-bezier(.30,0,.12,1);
          }
        }
        .music-interlude{
          padding-top: 0;
          padding-left: 25Px;
          width: 240Px;
          height: 0;
          opacity: 0;
          transform: scale(0);
          transition: 0.8s cubic-bezier(1,-0.49,.61,.36);
          display: flex;
          flex-direction: row;
          justify-content: center;
          align-items: center;
          position: relative;
          left: 0;
          .interlude-left{
            .diamond{
              width: 28Px;
              height: 28Px;
              border: 2Px solid black;
              transform: rotate(45deg);
              animation: diamond-rotate 1.6s 0.6s cubic-bezier(.30,0,.12,1) infinite;
              position: relative;
              @keyframes diamond-rotate {
                0%{transform: rotate(45deg);}
                50%{transform: rotate(135deg);}
                100%{transform: rotate(135deg);}
              }
              .diamond-inner{
                width: 85%;
                height: 85%;
                background-color: black;
                position: absolute;
                top: 50%;
                left: 50%;
                transform: translate(-50%, -50%);
              }
            }
          }
          .interlude-right{
            margin-left: 15Px;
            width: 100%;
            display: flex;
            flex-direction: column;
            overflow: hidden;
            position: relative;
            .triangle{
              width: 0;
              height: 0;
              border-top: 6Px solid black;
              border-left: 6Px solid transparent;
              position: absolute;
              top: 1Px;
              right: 0;
            }
            .remaining{
              font: 8Px SourceHanSansCN-Bold;
              color: black;
              white-space: nowrap;
            }
            .interlude-title{
              padding: 0 4Px;
              width: 100%;
              background-color: black;
              display: flex;
              flex-direction: row;
              align-items: center ;
              justify-content: space-between;
              white-space: nowrap;
              .title{
                font: 10Px SourceHanSansCN-Bold;
                color: white;
              }
              .title-style{
                width: 15%;
                height: 8Px;
                overflow: hidden;
              }
            }
            .interlude-progress{
              margin-top: 3Px;
              width: 100%;
              height: 4Px;
              background-color: black;
            }
          }
        }
        .music-interlude-in{
          padding-top: 10Px;
          height: 80Px;
          opacity: 1;
          transform: scale(1);
          transition: 0.8s cubic-bezier(.30,0,.12,1);
        }
      }
    }
    .lyric-area-hidden{
      transition: 0.2s cubic-bezier(.30,0,.12,1);
      transform: scale(0.85);
      opacity: 0;
    }
    .lyric-nodata{
      width: 100%;
      height: 100%;
      display: flex;
      flex-direction: row;
      justify-content: center;
      align-items: center;
      position: relative;
      .line1, .line2{
        width: 0;
        height: 0;
        position: absolute;
        background: 
        linear-gradient(
          to bottom right,
          rgba(0, 0, 0, 0) 0%,
          rgba(0, 0, 0, 0) calc(50% - 0.5px),
          rgba(0, 0, 0, 0.8) 50%,
          rgba(0, 0, 0, 0) calc(50% + 0.5px),
          rgba(0, 0, 0, 0) 100%
        );
        animation: nodata-open1 0.8s 0.5s cubic-bezier(.32,.81,.56,.98) forwards;
        @keyframes nodata-open1 {
        0%{width: 0;height: 0;}
        100%{width: 38%;height: 38%;}
        }
      }
      .tip{
        font: 16Px Bender-Bold;
        color: black;
        white-space: nowrap;
        opacity: 0;
        animation: nodata-open2 0.1s 1.3s forwards;
        @keyframes nodata-open2 {
          10%{opacity: 0;}
          20%{opacity: 1;}
          30%{opacity: 1;}
          40%{opacity: 0;}
          50%{opacity: 0;}
          60%{opacity: 1;}
          70%{opacity: 1;}
          80%{opacity: 0;}
          90%{opacity: 0;}
          100%{opacity: 1;}
        }
      }
      .line1{
        left: 4%;
        bottom: 4%;
      }
      .line2{
        top: 4%;
        right: 4%;
      }
    }
    .song-quality{
      font: 1.5vh Bender-Bold;
      color: black;
      position: absolute;
      bottom: -0.9vh;
      right: 1.5vh;
    }

    $boderPosition: -0.75 + vh;
    .border{
      width: 1.5vh;
      height: 1.5vh;
      border: 1Px solid black;
      position: absolute;
    }
    .border1{
      top: $boderPosition;
      left: $boderPosition;
    }
    .border2{
      top: $boderPosition;
      right: $boderPosition;
    }
    .border3{
      bottom: $boderPosition;
      right: $boderPosition;
      &::after{
        content: '';
        width: 0.5vh;
        height: 0.5vh;
        background-color: black;
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%,-50%);
      }
    }
    .border4{
      bottom: $boderPosition;
      left: $boderPosition;
    }
  }
  .fade-enter-active{
    transition: 0.4s cubic-bezier(.3,.79,.55,.99) !important;
  }
  .fade-leave-active {
    transition: 0.2s cubic-bezier(.3,.79,.55,.99) !important;
  }
  .fade-enter-from,
  .fade-leave-to {
    transform: scale(0.85);
    opacity: 0;
  }
</style>