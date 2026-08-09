import { defineStore } from "pinia";

const LYRIC_PREFERENCES = new Set(['original', 'trans', 'roma'])

function initialLyricPreferences() {
    if (typeof localStorage === 'undefined') return ['original']
    try {
        const persisted = JSON.parse(localStorage.getItem('playerStore') || 'null')
        const values = persisted?.lyricPreferences || persisted?.lyricType
        if (!Array.isArray(values)) return ['original']
        return [...new Set(values.filter((value) => LYRIC_PREFERENCES.has(value)))]
    } catch {
        return ['original']
    }
}

export const usePlayerStore = defineStore('playerStore', {
    state: () => {
        return {
            widgetState: true,//是否开启widget
            currentMusic: null,//播放列表的索引
            playing: false,//是否正在播放
            progress: 0,//进度条
            volume: 0.3,//音量
            // volumeBeforeMuted: 0,//静音前音量
            playMode: 0,//0为顺序播放，1为列表循环，2为单曲循环，3为随机播放
            listInfo: null,
            songList: null,//播放列表；null=尚未hydrate，[]=已hydrate但为空
            shuffledList: null,//随机播放列表
            shuffleIndex: 0,//随机播放列表的索引
            songId: null,
            currentIndex: 0,
            time: 0, //歌曲总时长
            playlistWidgetShow: false,
            playerChangeSong: false, //player页面切换歌曲更换歌名动画,
            lyricsObjArr: null,
            lyricsSynchronized: false,
            lyricSize: null,
            tlyricSize: null,
            rlyricSize: null,
            lyricPreferences: initialLyricPreferences(),
            lyricAvailability: { original: false, trans: false, roma: false },
            lyricInterludeTime: null, //歌词间奏等待时间
            lyricShow: false, //歌词是否显示
            lyricAnimationRevision: 0,
            isLyricDelay: true, //调整进度的时候禁止赋予delay属性
            localBase64Img: null, //如果是本地歌曲，获取封面
            forbidLastRouter: false, //在主动跳转router时禁用回到上次离开的路由的地址功能
            lyricBlur: false,
        }
    },
    getters: {
        hasPlaylist: (state) => Array.isArray(state.songList) && state.songList.length > 0,
    },
    actions: {
        applyParsedLyrics(parsed) {
            this.lyricsObjArr = parsed.lines.length ? parsed.lines : null
            this.lyricsSynchronized = parsed.synchronized
            this.lyricAvailability = { ...parsed.availability }
        },
        clearLyrics() {
            this.lyricsObjArr = null
            this.lyricsSynchronized = false
            this.lyricAvailability = { original: false, trans: false, roma: false }
        },
        toggleLyricPreference(type) {
            if (!LYRIC_PREFERENCES.has(type)) return
            this.lyricPreferences = this.lyricPreferences.includes(type)
                ? this.lyricPreferences.filter((value) => value !== type)
                : [...this.lyricPreferences, type]
        },
    },
    persist: {
        storage: localStorage,
        // 播放队列、歌曲、进度、音量和播放模式统一由 last-playlist.json 持久化，
        // 避免与 localStorage 形成两套 source of truth，并避免高频 progress 同步写入。
        paths: ['lyricPreferences','lyricBlur']
    },
})
