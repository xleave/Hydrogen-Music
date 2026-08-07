<script setup>
  import { songTime2 } from '../utils/player';
  import VueSlider from 'vue-slider-component'
  import PlayList from './PlayList.vue'
  import AppIcon from './icons/AppIcon.vue'
  import { startMusic, pauseMusic, playLast, playNext, changeProgress, changePlayMode } from '../utils/player'
  import { usePlayerStore } from '../store/playerStore'
  import { storeToRefs } from 'pinia';
  const playerStore = usePlayerStore()
  const { playing, progress, volume, playMode, currentIndex, songList, lyricType, playlistWidgetShow, time, playerChangeSong, localBase64Img, coverBlur } = storeToRefs(playerStore)
</script>

<template>
  <div class="player-container">
    <div class="player">
        <div class="player-cover">
            <div class="cover" :class="{'cover-change': playerChangeSong}">
                <img v-show="localBase64Img" :src="localBase64Img" alt="">
            </div>
            <div class="c-border c-border1"></div>
            <div class="c-border c-border2"></div>
            <div class="c-border c-border3"></div>
            <div class="c-border c-border4"></div>
        </div>
        <div class="player-info">
            <div class="info-music">
                <div class="music-name-lable" :class="{'music-name-lable-in': playerChangeSong}"></div>
                <span class="music-name" :class="{'music-name-in': playerChangeSong}">{{songList[currentIndex].name || songList[currentIndex].localName}}</span>
            </div>
            <div class="info-music">
                <div class="music-author-lable" :class="{'music-author-lable-video': coverBlur}"></div>
                <div class="music-author">
                    <span class="author" :style="{color: coverBlur ? 'black' : 'rgb(105, 105, 105)'}" v-for="(singer, index) in songList[currentIndex].ar">{{singer.name || ''}}{{index === songList[currentIndex].ar.length -1 ? '' : ' / '}}</span>
                </div>
            </div>
        </div>
        <div class="player-control">
            <div class="player-process">
                <div class="process-time">
                    <span class="time-current">{{songTime2(progress)}}</span>
                    <span class="time-end">{{songTime2(time)}}</span>
                </div>
                <div class="process">
                    <vue-slider id="widget-progress" class="music-progress" @click="changeProgress(progress)"  v-model="progress" :min="0" :max="time" :interval="1" :duration="0.5" tooltip="none"></vue-slider>
                </div>
            </div>

            <div class="control">
                <AppIcon name="previous" @click="playLast()" />
                <AppIcon v-show="playing" name="pause" @click="pauseMusic()" />
                <AppIcon v-show="!playing" name="play" @click="startMusic()" />
                <AppIcon name="next" @click="playNext()" />
            </div>

            <div class="player-voluem">
                <div class="voluem">
                    <vue-slider class="volume-slider" v-model="volume" :min="0" :max="1" :interval="0.01" :duration="0.3" tooltip="none"></vue-slider>
                </div>
                <div class="voluem-num">
                    <span class="voluem-title">VOLUME</span>
                    <span class="num">{{Math.round(volume * 100)}}</span>
                </div>
            </div>
        </div>

        <div class="song-control">
            <AppIcon v-show="lyricType.includes('roma') && !lyricType.includes('noRoma')" name="romanization" class="icon" @click="lyricType.splice(lyricType.indexOf('roma'), 1)" />
            <AppIcon v-show="!lyricType.includes('roma') && !lyricType.includes('noRoma')" name="romanization" class="icon inactive-icon" @click="lyricType.push('roma')" />
            <AppIcon v-show="lyricType.includes('trans') && !lyricType.includes('noTrans')" name="translation" class="icon" @click="lyricType.splice(lyricType.indexOf('trans'), 1)" />
            <AppIcon v-show="!lyricType.includes('trans') && !lyricType.includes('noTrans')" name="translation" class="icon inactive-icon" @click="lyricType.push('trans')" />
            <AppIcon v-show="lyricType.includes('original') && !lyricType.includes('noOriginal')" name="original" class="icon" @click="lyricType.splice(lyricType.indexOf('original'), 1)" />
            <AppIcon v-show="!lyricType.includes('original') && !lyricType.includes('noOriginal')" name="original" class="icon inactive-icon" @click="lyricType.push('original')" />
            
            <AppIcon v-show="playMode === 0" name="sequence" class="icon" @click="changePlayMode()" />
            <AppIcon v-show="playMode === 1" name="repeat" class="icon" @click="changePlayMode()" />
            <AppIcon v-show="playMode === 2" name="repeatOne" class="icon" @click="changePlayMode()" />
            <AppIcon v-show="playMode === 3" name="shuffle" class="icon" @click="changePlayMode()" />
            <AppIcon name="queue" class="playlist-icon" @click="playlistWidgetShow = !playlistWidgetShow" />
        </div>
    </div>
    
    <PlayList class="playlist-widget-player" :class="{'playlist-widget-open': playlistWidgetShow}"></PlayList>
    
    <span class="border border1"></span>
    <span class="border border2"></span>
    <span class="border border3"></span>
    <span class="border border4"></span>
  </div>
</template>

<style scoped lang="scss">
  .inactive-icon{
    color: #8a8a8a;
  }
  .player-container{
    position: relative;
    z-index: 99;
    &:hover{
      .song-control{
        animation: song-control 0.1s forwards;
        @keyframes song-control {
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
        svg{
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
    .player{
      width: 100%;
      height: 100%;
      transition: 0.2s;
      overflow: hidden;
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      .player-cover{
        width: 100%;
        transition: 0.2s cubic-bezier(.33,.88,.47,.94);
        position: relative;
        z-index: 99;
        .cover{
          padding: 1.5vh;
          width: 100%;
          opacity: 1;
          transform: scale(1);
          transition: 0.1s cubic-bezier(.3,.79,.55,.99);
          img{
            width: 100%;
            max-height: 38vh;
            object-fit: cover;
            vertical-align: bottom;
            box-shadow: 0 0 8Px 0 rgba(0, 0, 0, 0.05);
            transform: scale(1.03);
            animation: cover-in 0.3s 0.65s cubic-bezier(0.4, 0, 0.12, 1) forwards;
            @keyframes cover-in {
              0%{transform: scale(1.03);}
              100%{transform: scale(1);}
            }
          }
        }
        .cover-change{
          opacity: 0;
          transform: scale(0.95);
        }
        .back-Video{
          &:hover{
              cursor: pointer;
              transform: scale(1.05);
            }
        }
        $boderpx: 2 + Px;
        .c-border{
          width: 4vh;
          height: 4vh;
          position: absolute;
        }
        .c-border1{
          top: 1vh;
          left: 1vh;
          border: {
            top: $boderpx solid black;
            left: $boderpx solid black;
          };
          animation: border1 0.3s 0.65s cubic-bezier(0.4, 0, 0.12, 1) forwards;
          @keyframes border1 {
            0%{top: 1vh;left: 1vh;}
            100%{top: 0;left: 0;}
          }
        }
        .c-border2{
          top: 1vh;
          right: 1vh;
          border: {
            top: $boderpx solid black;
            right: $boderpx solid black;
          };
          animation: border2 0.2s 0.65s cubic-bezier(0.4, 0, 0.12, 1) forwards;
          @keyframes border2 {
            0%{top: 1vh;right: 1vh;}
            100%{top: 0;right: 0;}
          }
        }
        .c-border3{
          bottom: 1vh;
          right: 1vh;
          border: {
            bottom: $boderpx solid black;
            right: $boderpx solid black;
          };
          animation: border3 0.3s 0.65s cubic-bezier(0.4, 0, 0.12, 1) forwards;
          @keyframes border3 {
            0%{bottom: 1vh;right: 1vh;}
            100%{bottom: 0;right: 0;}
          }
        }
        .c-border4{
          bottom: 1vh;
          left: 1vh;
          border: {
            bottom: $boderpx solid black;
            left: $boderpx solid black;
          };
          animation: border4 0.3s 0.65s cubic-bezier(0.4, 0, 0.12, 1) forwards;
          @keyframes border4 {
            0%{bottom: 1vh;left: 1vh;}
            100%{bottom: 0;left: 0;}
          }
        }
      }
      .player-info{
        margin-top: 1vh;
        padding: 1.5vh;
        width: 100%;
        .info-music{
          width: 100%;
          display: flex;
          flex-direction: row;
          justify-content: flex-start;
          align-items: center;
          text-align: left;
          white-space: nowrap;
          position: relative;
          &:first-child{
            padding-bottom: 1.2vh;
            overflow: hidden;
          }
          .music-name-lable,.music-author-lable{
            position: absolute;
          }
          .music-name,.music-author{
            margin-left: 10Px;
            width: 100%;
            font-family: SourceHanSansCN-Bold;
            user-select: text;
            &::-webkit-scrollbar{
                display: none;
            }
          }
          .music-name{
            margin-left: 1.5vh;
            overflow: hidden;
            white-space: nowrap;
            text-overflow: ellipsis;
          }
          .music-name-lable{
            width: 100%;
            height: 2.9vh;
            background-color: black;
            transition: 0.3s cubic-bezier(.22,.89,.58,.99);
            transform: translateX(calc(-100% + 5Px));
          }
          .music-name-lable-in{
            transform: translateX(0);
          }
          .music-name{
            padding: 0.3vh 0;
            font-family: SourceHanSansCN-Bold;
            font-weight: bold;
            font-size: 2.4vh;
            color: black;
          }
          .music-name-in{
            opacity: 0;
          }
          .music-author-lable{
            width: 8Px;
            height: 8Px;
            border: 0.5Px solid rgb(105, 105, 105);
            position: absolute;
            top: 1Px;
            left: -2Px;
            &::after{
              content: '';
              width: 4Px;
              height: 4Px;
              background-color: rgb(105, 105, 105);
              position: absolute;
              top: 50%;
              left: 50%;
              transform: translate(-50%,-50%);
            }
          }
          .music-author-lable-video{
            border: 0.5Px solid rgb(0, 0, 0);
            &::after{
              background-color: rgb(0, 0, 0);
            }
          }
          .music-author{
            font-size: 10Px;
            color: rgb(105, 105, 105);
            .author{
                transition: 0.2s;
                &:hover{
                  cursor: pointer;
                  color: black !important;
                }
            }
          }
        }
      }
      .player-control{
        padding: 1.5vh;
        height: 32%;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        .player-process{
          .process-time{
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            align-items: center;
            font: 1.5vh Bender-Bold;
            color: black;
          }
          .process{
              width: 100%;
              height: 1.3vh;
              position: relative;
              .music-progress{
                width: 100% !important;
                height: 1.3vh !important;
                box-shadow: 0 0 0 0.5Px black;
                transition: 0.2s;
              }
          }
        }
        .control{
          // margin: 2vh 0;
          display: flex;
          flex-direction: row;
          justify-content: space-evenly;
          align-items: center;
          svg{
            width: 5vh;
            height: 5vh;
            transition: 0.2s;
            &:hover{
                cursor: pointer;
            }
            &:active{
                transform: scale(0.90);
            }
          }
        }
        .player-voluem{
          .voluem{
            width: 100%;
            height: 1.3vh;
            position: relative;
            .volume-slider{
              height: 1.3vh !important;
              box-shadow: 0 0 0 0.5Px black !important;
            }
            .voluem-outline{
              width: 100%;
              height: 100%;
              border: 1Px solid black;
              position: absolute;
            }
            .voluem-content{
              width: 46%;
              height: 100%;
              background-color: black;
              position: absolute;
            }
          }
          .voluem-num{
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            align-items: center;
            font: 1.5vh Bender-Bold;
            color: black;
          }
        }
      }
      .song-control{
        width: 50Px;
        display: flex;
        flex-direction: column;
        align-items: center;
        position: absolute;
        bottom: 2vh;
        right: -50Px;
        opacity: 0;
        svg{
          margin-top: 3vh;
          width: 2.5vh;
          height: 2.5vh;
        }
      }
    }
    .playlist-widget-player{
      position: absolute;
      right: -370Px;
      bottom: 0;
    }
    .playlist-widget-open{
      height: 450Px;
    }
    $boderPosition: -0.75 + vh;
    .border{
      width: 1.5vh;
      height: 1.5vh;
      background-color: black;
      position: absolute;
      z-index: 100;
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
    }
    .border4{
      bottom: $boderPosition;
      left: $boderPosition;
    }
  }
</style>
