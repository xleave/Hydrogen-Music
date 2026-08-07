import pinia from '../store/pinia'
import { useLocalStore } from '../store/localStore'
import { storeToRefs } from 'pinia'
import { nanoid } from 'nanoid'
import { noticeOpen } from './dialog'

const localStore = useLocalStore(pinia)
const { localMusicFolder, localMusicList, localMusicClassify, isRefreshLocalFile } = storeToRefs(localStore)

// 使用 Map 实现 O(1) 查找，替代原来的 O(n) findIndex
let artistMap = new Map()
let albumMap = new Map()

function classifyAdd(song) {
    // Rust 端已完成艺术家分割，直接使用
    const artists = song.common.artists?.length ? song.common.artists : ['其他']
    artists.forEach(artist => {
        if (!artistMap.has(artist)) {
            artistMap.set(artist, {
                id: nanoid(),
                type: 'artist',
                name: artist,
                songs: []
            })
        }
        artistMap.get(artist).songs.push(song)
    })

    const album = song.common.album || '其他'
    if (!albumMap.has(album)) {
        albumMap.set(album, {
            id: nanoid(),
            type: 'album',
            name: album,
            songs: []
        })
    }
    albumMap.get(album).songs.push(song)
}

function classify(arr) {
    arr.forEach(item => {
        if (item.children) classify(item.children)
        else classifyAdd(item)
    })
    return {
        artists: Array.from(artistMap.values()),
        albums: Array.from(albumMap.values())
    }
}

export function scanMusic(params) {
    if(isRefreshLocalFile.value)
        noticeOpen("正在扫描本地音乐,请稍等", 3)
    windowApi.scanLocalMusic(params)
}
windowApi.localMusicCount((event, count) => {
    noticeOpen('已扫描' + count + '首', 2)
})
windowApi.localMusicFiles((event, localData) => {
    if(localData.type == 'local') {
        localMusicFolder.value = localData.dirTree
        localMusicList.value = localData.locaFilesMetadata
        // 重置 Map，避免多次扫描累积
        artistMap = new Map()
        albumMap = new Map()
        localMusicClassify.value = classify(localData.locaFilesMetadata)
    }
    if(isRefreshLocalFile.value) {
        noticeOpen("扫描完毕 共" + localData.count + '首', 3)
        isRefreshLocalFile.value = false
    }
})
