import pinia from '../store/pinia'
import { Howl, Howler } from 'howler'
import { convertFileSrc } from '@tauri-apps/api/core'
import dayjs from 'dayjs';
import { noticeOpen } from './dialog'
import { usePlayerStore } from '../store/playerStore'
import { useOtherStore } from '../store/otherStore'
import { storeToRefs } from 'pinia'
import duration from 'dayjs/plugin/duration'

const otherStore = useOtherStore()
const playerStore = usePlayerStore(pinia)
const { currentMusic, playing, progress, volume, playMode, songList, shuffledList, shuffleIndex, listInfo, songId, currentIndex, time, playlistWidgetShow, playerChangeSong, lyric, lyricsObjArr, lyricShow, lyricEle, isLyricDelay, widgetState, localBase64Img, lyricBlur, coverUrl} = storeToRefs(playerStore)

let isProgress = false
let musicProgress = null
let loadLast = true
let playModeOne = false //为true代表顺序播放已全部结束

export function loadLastSong() {
    if(loadLast) {
        windowApi.getLastPlaylist().then(list => {
            if(list) {
                songList.value = list.songList
                shuffledList.value = list.shuffledList
            }
            if(songList.value && songList.value.length > 0 && currentIndex.value < songList.value.length) {
                if(songList.value[currentIndex.value].type == 'local') getSongUrl(songList.value[currentIndex.value].id, currentIndex.value, false, true)
                else getSongUrl(songList.value[currentIndex.value].id, currentIndex.value, false, false)
            } else {
                // 无上次播放记录（全新安装），直接重置 loadLast，避免首次播放时被静音
                loadLast = false
            }
        })
    }
}

export function play(url, autoplay) {
    if(currentMusic.value) {
        currentMusic.value.unload()
        Howler.unload()
    }
    // 捕获当前的 loadLast 状态，避免 load/play 事件竞态问题
    const isResumingLastSong = loadLast
    if(isResumingLastSong) {
        loadLast = false
    }
    currentMusic.value = new Howl({
        src: url,
        autoplay: autoplay,
        html5: true,
        preload: true,
        format: ['mp3', 'flac', 'wav', 'aac', 'm4a', 'ogg', 'opus'],
        loop: (playMode.value == 2),
        volume: isResumingLastSong ? 0 : volume.value,
        xhr: {
            method: 'GET',
            withCredentials: true,
        },
        onend: function() {
            clearInterval(musicProgress)
            if(playMode.value == 0 && currentIndex.value < songList.value.length - 1) { playNext();return } //顺序播放
            if(playMode.value == 0 && currentIndex.value == songList.value.length - 1) { playing.value = false;playModeOne = true;windowApi.playOrPauseMusicCheck(playing.value);return } //顺序播放结束暂停状态
            if(playMode.value == 1) { playNext();return } //列表循环
            if(playMode.value == 3) { playNext() } //随机播放(为列表循环)
            if(playMode.value == 2) { clearLycAnimation() } // 单曲循环播放结束时清除歌词动画
        }
    })
    currentMusic.value.once('load', () => {
        time.value = Math.floor(currentMusic.value.duration())
        if(isResumingLastSong) {
            // 恢复上次播放进度：seek 到上次位置，音量已在初始化时设为0
            currentMusic.value.seek(progress.value)
        }
        playerChangeSong.value = false
    })
    currentMusic.value.on('play', () => {
        currentMusic.value.fade(0, volume.value, 200)
        startProgress()
        playing.value = true
        windowApi.playOrPauseMusicCheck(playing.value)
    })
    currentMusic.value.on('pause', () => {
        clearInterval(musicProgress)
        playing.value = false
        windowApi.playOrPauseMusicCheck(playing.value)
        currentMusic.value.fade(volume.value, 0, 200)
    })
}

export function startProgress() {
    clearInterval(musicProgress)
    progress.value = currentMusic.value.seek()
    musicProgress = setInterval(() => {
        if(currentMusic.value.seek() < time.value)
            progress.value = currentMusic.value.seek()
    }, 1000);
}

export function setId(id, index) {
    if(playMode.value != 3) {
        songId.value = id
        currentIndex.value = index
    } else {
        songId.value = id
        shuffleIndex.value = index
        currentIndex.value = (songList.value || []).findIndex((song) => song.id === songId.value)
    }
}

export function addToList(listType, songlist) {
    listInfo.value = {
        id: 'local',
        type: listType
    }
    songList.value = songlist.slice(0,songlist.length + 1)
    savePlaylist()
}

export function localMusicHandle(list, isToNext) {
    let addList = []
    list.forEach(song => {
        let ar = []
        if(song.common.artists)
            song.common.artists.forEach(artist => {
                ar.push({
                    id: 'local',
                    name: artist
                })
            })
        else {
            ar.push({
                id: 'local',
                name: 'NONE'
            })
        }
        addList.push(
            {
                id: song.id,
                ar: ar,
                url: song.dirPath,
                name: song.common.title,
                localName: song.common.localTitle,
                type: 'local',
                sampleRate: song.format.sampleRate / 1000,
                bitsPerSample: song.format.bitsPerSample,
                bitrate: Math.round(song.format.bitrate / 1000),
            }
        )
    });
    if(isToNext) return addList[0]
    return addList
}

export function addLocalMusicTOList(listType, localMusicList, playId, playIndex) {
    listInfo.value = {
        id: 'local',
        type: listType
    }
    
    songList.value = localMusicHandle(localMusicList, false)
    addSong(playId, playIndex, true, true)
    savePlaylist()
}
export function addSong(id, index, autoplay, isLocal) {
    progress.value = 0
    if(lyricShow.value) {
        lyricShow.value = false
        playerChangeSong.value = true
    }
    setId(id, index)
    isLocal = true
    
    if(currentMusic.value && volume.value != 0) {
        const state = currentMusic.value.state()
        if(state == 'loading' || state == 'unloaded') {
            // 当前音乐处于加载/未加载状态，直接卸载并播放新歌，无需等待 fade
            currentMusic.value.unload()
            getSongUrl(id, index, autoplay, isLocal)
        } else {
            currentMusic.value.fade(volume.value, 0, 200)
            currentMusic.value.once('fade', () => {
                getSongUrl(id, index, autoplay, isLocal)
            })
        }
    } else {
        getSongUrl(id, index, autoplay, isLocal)
    }
}

export function setSongLevel(level) {
    if(level == 'standard') songList.value[currentIndex.value].level = songList.value[currentIndex.value].l
    else if(level == 'higher') songList.value[currentIndex.value].level = songList.value[currentIndex.value].m
    else if(level == 'exhigh') songList.value[currentIndex.value].level = songList.value[currentIndex.value].h
    else if(level == 'lossless') songList.value[currentIndex.value].level = songList.value[currentIndex.value].sq
    else if(level == 'hires') songList.value[currentIndex.value].level = songList.value[currentIndex.value].hr
    songList.value[currentIndex.value].quality = level
}
export async function getLocalLyric(filePath) {
    const lyric = await windowApi.getLocalMusicLyric(filePath)
    if(lyric) return lyric
    else return false
}
export function setSongToWindows() {
    if(songList.value[currentIndex.value].type != 'local') {
        coverUrl.value = songList.value[currentIndex.value].al.picUrl + '?param=128y128'
    } else {
        if(!localBase64Img.value) coverUrl.value = null
        else coverUrl.value = localBase64Img.value
    }
    if ('mediaSession' in navigator) {
        navigator.mediaSession.metadata = new MediaMetadata({
          title: [songList.value[currentIndex.value].name],
          artist: [songList.value[currentIndex.value].ar.map(a => a.name)],
          artwork: [
            { src: coverUrl.value }
          ]
        });
    } else {
        return
    }
}
export async function getSongUrl(id, index, autoplay, isLocal) {
    windowApi.setWindowTile(songList.value[currentIndex.value].name + " - " + songList.value[currentIndex.value].ar[0].name)
    if(isLocal) {
        windowApi.getLocalMusicImage(songList.value[currentIndex.value].url).then(base64 => {
            localBase64Img.value = base64
            setSongToWindows()
        })
        play(convertFileSrc(songList.value[currentIndex.value].url), autoplay)
        lyric.value = null
        lyricsObjArr.value = null
        const localLyric = await getLocalLyric(songList.value[currentIndex.value].url)
        if(localLyric) lyric.value = {lrc:{lyric:localLyric}}
        if(!lyricShow.value && !widgetState.value) {
            lyricShow.value = true
            playerChangeSong.value = false
        }
        return
    }
    noticeOpen('本地版仅播放本地音乐', 2)
}

export function startMusic() {
    if(playMode.value == 0 && currentIndex.value == songList.value.length - 1 && playModeOne && currentMusic.value.seek() == 0) {playNext();playModeOne = false;return}
    if(!playing.value) {
        currentMusic.value.play()
    }
    if(lyricShow.value) {
        isLyricDelay.value = false
        const forbidDelayTimer =  setTimeout(() => {
            isLyricDelay.value = true
            clearTimeout(forbidDelayTimer)
        }, 700);
    }
}
export function pauseMusic() {
    clearInterval(musicProgress)
    if(playing.value) {
        currentMusic.value.fade(volume.value,0,200)
        currentMusic.value.once('fade', () => {
            currentMusic.value.pause()
            playing.value = false
        })
    }
}

export function playLast() {
    let id = null
    let index = null
    if(playMode.value != 3) {
        if(currentIndex.value - 1 < 0) {
            index = songList.value.length - 1
            id = songList.value[index].id
        } else {
            id = songList.value[currentIndex.value - 1].id
            index = currentIndex.value - 1
        }
    }
    if(playMode.value == 3) {
        if(shuffleIndex.value - 1 < 0) {
            index = shuffledList.value.length - 1
            id = shuffledList.value[index].id
        } else {
            index = shuffleIndex.value - 1
            id = shuffledList.value[index].id
        }
    }
    addSong(id, index, true)
}
export function playNext() {
    let id = null
    let index = null
    if(playMode.value != 3) {
        if(songList.value.length - 1 == currentIndex.value) {
            index = 0
            id = songList.value[index].id
        } else {
            index = currentIndex.value + 1
            id = songList.value[index].id
        }
    }
    if(playMode.value == 3) {
        if(shuffleIndex.value == shuffledList.value.length - 1) {
            index = 0
            id = shuffledList.value[index].id
        } else {
            index = shuffleIndex.value + 1
            id = shuffledList.value[index].id
        }
    }
    addSong(id, index, true)
}
const clearLycAnimation = () => {
    isLyricDelay.value = false
    for (let i = 0; i < lyricEle.value.length; i++) {
      lyricEle.value[i].style.transitionDelay = 0 + 's'
      if(lyricBlur.value) lyricEle.value[i].firstChild.style.setProperty("filter", "blur(0)");
    }
    const forbidDelayTimer =  setTimeout(() => {
        isLyricDelay.value = true
        clearTimeout(forbidDelayTimer)
    }, 600);
  }
export function changeProgress(toTime) {
    if(!widgetState.value && lyricShow.value && lyricEle.value) clearLycAnimation()
    currentMusic.value.seek(toTime)
}
//控制拖拽进度条
export function changeProgressByDragStart() {
    clearInterval(musicProgress)
}
export function changeProgressByDragEnd(toTime) {
    changeProgress(toTime)
    if(playing.value) startProgress()
}
// ------------
export function changePlayMode() {
    if(playMode.value != 3) playMode.value += 1
    else playMode.value = 0

    if(playMode.value == 2) currentMusic.value.loop(true) //循环模式
    else currentMusic.value.loop(false)
    if(playMode.value == 3) {
        setShuffledList()
    } else {
        shuffledList.value = null
        shuffleIndex.value = null
    }
    windowApi.changeTrayMusicPlaymode(playMode.value)
}

export function playAll(listType, list) {
    if(playMode.value == 3) {
        addToList(listType, list)
        setShuffledList(true)
        addSong(shuffledList.value[0].id, 0, true)
    } else {
        addToList(listType, list)
        addSong(songList.value[0].id, 0, true)
    }
}

export function setShuffledList(isplayAll) { 
    shuffledList.value = shuffle(songList.value, isplayAll)
    shuffleIndex.value = 0
 }

function shuffle(arr, isplayAll) { // 随机打乱数组
    let _arr = arr.slice() // 调用数组副本，不改变原数组
    for (let i = 0; i < _arr.length; i++) {
      let j = getRandomInt(0, i)
      let t = _arr[i]
      _arr[i] = _arr[j]
      _arr[j] = t
    }
    if(!isplayAll) {
        let currentSongIndex = (_arr || []).findIndex((song) => song.id === songId.value) //在打乱的列表中找到当前播放歌曲删除并添加至队列顶部
        _arr.splice(currentSongIndex, 1)
        _arr.unshift(songList.value[currentIndex.value])
    }
    return _arr
  }
function getRandomInt(min, max) { // 获取min到max的一个随机数，包含min和max本身
    return Math.floor(Math.random() * (max - min + 1) + min)
}

export function addToNext(nextSong, autoplay) {
    if(!songList.value) songList.value = []
    if(nextSong.id == songId.value) return

    const si = (songList.value || []).findIndex((song) => song.id === nextSong.id)
    if(si != -1) {
        songList.value.splice(si, 1)
        if(si < currentIndex.value) currentIndex.value--
    }
    songList.value.splice(currentIndex.value + 1, 0, nextSong)

    if(playMode.value == 3) {
        const shufflei = (shuffledList.value || []).findIndex((song) => song.id === nextSong.id)
        if(shufflei != -1) {
            shuffledList.value.splice(shufflei, 1)
            if(shufflei < currentIndex.value) shuffleIndex.value--
        }
        shuffledList.value.splice(shuffleIndex.value + 1, 0, nextSong)
    }
    if(autoplay) playNext()
    else noticeOpen('已添加至下一首', 2)
    if(songList.value.length == 1) addSong(nextSong.id, 0, autoplay)
}
export function addToNextLocal(song, autoplay) {
    addToNext(localMusicHandle([song], true), autoplay)
}
export function savePlaylist() {
    let list = {
        songList: songList.value,
        shuffledList: shuffledList.value
    }
    windowApi.saveLastPlaylist(JSON.stringify(list))
}
export function songTime(dt) {
    dayjs.extend(duration)
    if(dt) {
        if ( dt == 0 || dt == "--") return dt;
        const day = dayjs.duration(dt)
        let str = "";
        if (day.minutes() >= 0) str += day.minutes() + ':';
        str += day.seconds().toString().padStart(2, '0')
        return str;
    }
}
export function songTime2(time) {
    let min = Math.floor(time / 60)
    let sec = Math.floor(time % 60)
    if(sec == 60) {
        sec = 0
        min++
    }
    if(min < 10) min = '0' + min
    if(sec < 10) sec = '0' + sec
    return min + ':' + sec
}
window.addEventListener('mousedown', (e) => {
    if(e.target.parentNode.parentNode.id == 'widget-progress') {
      changeProgressByDragStart()
      isProgress = true
    }
})

window.addEventListener('mouseup', () => {
  if(isProgress) {
      changeProgressByDragEnd(progress.value)
      isProgress = false
  }
})
  
window.addEventListener('click', (e) => {
  if(playlistWidgetShow.value) {
      if(document.getElementsByClassName('playlist-widget')[0].contains(e.target) == false && document.getElementsByClassName('music-control')[0].contains(e.target) == false && document.getElementsByClassName('music-other')[0].contains(e.target) == false && document.getElementsByClassName('playlist-widget-player')[0].contains(e.target) == false && document.getElementsByClassName('song-control')[0].contains(e.target) == false && document.getElementsByClassName('contextMune')[0].contains(e.target) == false && e.target.className.baseVal != 'item-delete') 
        playlistWidgetShow.value = false
  }
  if(otherStore.contextMenuShow) otherStore.contextMenuShow = false
})
windowApi.playOrPauseMusic((event) => {
    if(playing.value) pauseMusic()
    else startMusic()
})
windowApi.lastOrNextMusic((event, option) => {
    if(option == 'last') playLast()
    else if(option == 'next') playNext()
})
windowApi.changeMusicPlaymode((event, mode) => {
    if(playMode.value != mode) playMode.value = mode
    if(playMode.value == 2) currentMusic.value.loop(true) //循环模式
    else currentMusic.value.loop(false)
    if(playMode.value == 3) {
        setShuffledList()
    } else {
        shuffledList.value = null
        shuffleIndex.value = null
    }
})
windowApi.volumeUp(() => {
    if(volume.value + 0.1 < 1) volume.value += 0.1
    else volume.value = 1
    currentMusic.value.volume(volume.value)
})
windowApi.volumeDown(() => {
    if(volume.value - 0.1 > 0) volume.value -= 0.1
    else volume.value = 0
    currentMusic.value.volume(volume.value)
})
windowApi.musicProcessControl((event, mode) => {
    if(mode == 'forward') {
        if(progress.value + 3 < currentMusic.value.duration()) progress.value += 3
        else progress.value = currentMusic.value.duration()
    } else if(mode == 'back') {
        if(progress.value - 3 > 0) progress.value -= 3
        else progress.value = 0
    }
    currentMusic.value.seek(progress.value)
})
windowApi.playOrPauseMusicCheck(playing.value)
windowApi.changeTrayMusicPlaymode(playMode.value)
windowApi.beforeQuit(() => {
    let list = {
        songList: songList.value,
        shuffledList: shuffledList.value
    }
    windowApi.exitApp(JSON.stringify(list))
})
if ('mediaSession' in navigator) {
    navigator.mediaSession.setActionHandler('previoustrack', () => {
      playLast()
    });
    navigator.mediaSession.setActionHandler('nexttrack', () => {
      playNext()
    });
    navigator.mediaSession.setActionHandler('play', () => {
       startMusic();
    });
    navigator.mediaSession.setActionHandler('pause', () => {
      pauseMusic()
    });
}
