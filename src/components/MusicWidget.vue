<script setup>
  import { ref, watch} from 'vue'
  import { songTime2 } from '../utils/player';
  import VueSlider from 'vue-slider-component'
  import '../assets/css/slider.css'
  import PlayList from './PlayList.vue'
  import AppIcon from './icons/AppIcon.vue'

  import { startMusic, pauseMusic, playLast, playNext, changeProgress, changePlayMode } from '../utils/player'
  import { usePlayerStore } from '../store/playerStore'
  import { storeToRefs } from 'pinia'
  const playerStore = usePlayerStore()
  const { currentMusic, playing, progress, playMode, songList, currentIndex, volume, time, playlistWidgetShow, lyricShow, localBase64Img } =storeToRefs(playerStore)
  const showMusicTime = ref(false)

  watch(() => volume.value, () => {
    currentMusic.value?.volume(volume.value)
  })

  const showPlayer = () => {
    playerStore.widgetState = false
    lyricShow.value = true
  }

</script>
<template>
  <div class="music-widget">
    <div class="music-progress-container">
        <vue-slider id="widget-progress" class="music-progress" @click="changeProgress(progress)"  v-model="progress" :min="0" :max="time" :interval="1" :duration="0.5" tooltip="none"></vue-slider>
        <div class="music-time">{{songTime2(progress)}} / {{songTime2(time)}}</div>
    </div>
    <div class="music-info">
        <div class="music-img" @click="showPlayer()">
            <img v-show="localBase64Img" :src="localBase64Img" alt="">
            <div class="open-player">
                <AppIcon name="expand" class="open-player-icon" />
            </div>
        </div>
        <div class="music-info-other">
            <span class="music-name" :class="{'music-time-in': showMusicTime}">{{songList[currentIndex].name || songList[currentIndex].localName}}</span>
            <div class="music-author">
                <span class="author" v-for="(singer, index) in songList[currentIndex].ar">{{singer.name || ''}}{{index === songList[currentIndex].ar.length -1 ? '' : ' / '}}</span>
            </div>
        </div>
    </div>
    <div class="music-right">
        <div class="music-control">
            <AppIcon name="previous" class="control-icon" @click="playLast()" />
            <AppIcon v-show="playing" name="pause" class="control-icon" @click="pauseMusic()" />
            <AppIcon v-show="!playing" name="play" class="control-icon" @click="startMusic()" />
            <AppIcon name="next" class="control-icon" @click="playNext()" />
        </div>
        <div class="music-volume">
            <div class="volume-container">
                <vue-slider class="volume-slider" v-model="volume" :min="0" :max="1" :interval="0.01" :duration="0.3" tooltip="none"></vue-slider>
                <div class="volume-info">
                    <div class="volume-lable">VOLUME</div>
                    <div class="volume-num">{{Math.round(volume * 100)}}</div>
                </div>
            </div>
        </div>
        <div class="music-other">
            <AppIcon v-show="playMode === 0" name="sequence" class="icon" @click="changePlayMode()" />
            <AppIcon v-show="playMode === 1" name="repeat" class="icon" @click="changePlayMode()" />
            <AppIcon v-show="playMode === 2" name="repeatOne" class="icon" @click="changePlayMode()" />
            <AppIcon v-show="playMode === 3" name="shuffle" class="icon" @click="changePlayMode()" />
            <AppIcon name="queue" class="playlist-icon" @click="playlistWidgetShow = !playlistWidgetShow" />
        </div>
    </div>
    <PlayList class="playlist-widget" :class="{'playlist-widget-open': playlistWidgetShow}"></PlayList>
    <div class="widget-back"></div>
  </div>
</template>

<style scoped lang="scss">
  .music-widget{
    width: 100%;
    height: 100%;
    background-color: rgba(225, 240, 240, 1);
    position: relative;
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    .music-progress-container{
        width: 100%;
        height: 8Px;
        position: absolute;
        top: -2.5Px;
        &:hover{
            .music-progress{
                height: 13Px !important;
            }
            .music-time{
                opacity: 1;
                pointer-events: none;
            }
        }
        .music-progress{
            width: 100% !important;
            height: 2.5Px !important;
            background-color: rgb(223, 223, 223);
            transition: 0.2s;
        }
        .music-time{
            padding: 0 2Px;
            font: 9Px Bender-Bold;
            color: white;
            position: absolute;
            top: 0;
            left: 0;
            z-index: 999;
            opacity: 0;
            transition: 0.2s;
        }
    }
    .music-info{
        margin-left: 17Px;
        display: flex;
        flex-direction: row;
        .music-img{
            width: 45Px;
            height: 45Px;
            position: relative;
            border: 0.5Px solid rgba(0, 0, 0, 0.1);
            img{
                width: 100%;
                height: 100%;
            }
            .open-player{
                width: 100%;
                height: 100%;
                overflow: hidden;
                position: absolute;
                top: 0;
                left: 0;
                transition: 0.2s;
                .open-player-icon{
                    width: 40%;
                    height: 40%;
                    position: absolute;
                    top: 120%;
                    left: 50%;
                    transform: translate(-50%, -50%);
                    transition: 0.2s cubic-bezier(0,1.06,.77,.99);
                }
                &:hover{
                    cursor: pointer;
                    background-color: rgba(0, 0, 0, 0.5);
                    .open-player-icon{
                        top: 50%;
                    }
                }
            }
        }
        .music-info-other{
            margin-left: 8Px;
            width: 175Px;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: flex-start;
            user-select: text;
            .music-name,.music-author{
                text-align: left;
                overflow: hidden;
                display: -webkit-box;
                -webkit-box-orient: vertical;
                -webkit-line-clamp: 1;
                word-break: break-all;
            }
            .music-name{
                font: 14Px SourceHanSansCN-Bold;
                font-weight: bold;
                color: black;
            }
            .music-author{
                font: 10Px SourceHanSansCN-Bold;
                color: rgb(131, 131, 131);
                .author{
                    transition: 0.2s;
                    &:hover{
                        cursor: pointer;
                        color: black;
                    }
                }
            }
        }
    }
    .music-right{
        display: flex;
        flex-direction: row;
        align-items: center;
        .music-control{
            padding: 0 18Px;
            display: flex;
            flex-direction: row;
            align-items: center;
            svg{
                width: 20Px;
                height: 20Px;
                transition: 0.2s;
                &:hover{
                    cursor: pointer;
                }
                &:active{
                    transform: scale(0.90);
                }
                &:nth-child(2),&:nth-child(3){
                    margin: 0 15Px;
                }
            }
        }
        .music-volume{
            width: 120Px;
            .volume-container{
                width: 100%;
                height: 7Px;
                position: relative;
                .volume-slider{
                    height: 7Px !important;
                    box-shadow: 0 0 0 0.5Px black !important;
                }
                .volume-process-outline{
                    width: 100%;
                    height: 100%;
                    border: 1Px solid black;
                    position: absolute;
                    top: 0;
                    left: 0;
                }
                .volume-process{
                    width: 64%;
                    height: 100%;
                    background: black;
                    position: absolute;
                    top: 0;
                    left: -1Px;
                }
                .volume-info{
                    display: flex;
                    flex-direction: row;
                    align-items: center;
                    position: absolute;
                    top: -10Px;
                    left: 0;
                    .volume-lable,.volume-num{
                        font: 8Px Geometos;
                    }
                    .volume-lable{
                        margin-right: 6Px;
                        color: rgb(106, 106, 106);
                    }
                }
            }
        }
        .music-other{
            margin-left: 20Px;
            display: flex;
            flex-direction: row;
            align-items: center;
            svg{
                margin-right: 22Px;
                width: 20Px;
                height: 20Px;
                transition: 0.2s;
                &:hover{
                    cursor: pointer;
                }
                &:active{
                    transform: scale(0.90);
                }
            }
        }
    }
    .playlist-widget{
        position: absolute;
        bottom: 75Px;
        right: 0;
    }
    .playlist-widget-open{
        height: 450Px;
    }
    .widget-back{
        width: 5Px;
        height: 5Px;
        border-radius: 50%;
        background-color: rgba(160, 160, 160, 0.7);
        position: absolute;
        top: 6Px;
        right: 6Px;
    }
  }
</style>
