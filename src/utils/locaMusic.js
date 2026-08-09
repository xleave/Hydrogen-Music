import pinia from '../store/pinia'
import { useLocalStore } from '../store/localStore'
import { useLibraryStore } from '../store/libraryStore'
import { storeToRefs } from 'pinia'
import { noticeOpen } from './dialog'
import { restorePlaylistFromLibrary } from './player'
import { libraryEntityId } from './libraryEntityId.mjs'

const localStore = useLocalStore(pinia)
const libraryStore = useLibraryStore(pinia)
const { isRefreshLocalFile } = storeToRefs(localStore)

let artistMap = new Map()
let albumMap = new Map()

function classifyAdd(song) {
    const artists = song.common.artists?.length ? song.common.artists : ['其他']
    artists.forEach((artist) => {
        if (!artistMap.has(artist)) {
            artistMap.set(artist, {
                id: libraryEntityId('artist', [artist]),
                type: 'artist',
                name: artist,
                trackIds: []
            })
        }
        artistMap.get(artist).trackIds.push(song.id)
    })

    const album = song.common.album || '其他'
    const albumArtist = song.common.albumartist || artists[0] || '其他'
    const albumId = JSON.stringify([albumArtist, album])
    if (!albumMap.has(albumId)) {
        albumMap.set(albumId, {
            id: libraryEntityId('album', [albumArtist, album]),
            type: 'album',
            name: album,
            albumArtist,
            trackIds: []
        })
    }
    albumMap.get(albumId).trackIds.push(song.id)
}

function classify(arr) {
    for (const item of arr || []) {
        if (item.children) classify(item.children)
        else if (item.common) classifyAdd(item)
    }
    return {
        artists: Array.from(artistMap.values()),
        albums: Array.from(albumMap.values())
    }
}

export function scanMusic(params) {
    if (isRefreshLocalFile.value) noticeOpen('正在扫描本地音乐,请稍等', 3)
    return windowApi.scanLocalMusic({
        ...params,
        knownRevision: localStore.libraryRevision,
    }).catch((error) => {
        console.error('[local scan]', error)
        if (isRefreshLocalFile.value) isRefreshLocalFile.value = false
        noticeOpen('本地音乐扫描失败', 3)
    })
}

windowApi.localMusicCount((event, count) => {
    noticeOpen('已扫描' + count + '首', 2)
})

windowApi.localMusicFiles((event, localData) => {
    if (localData.type === 'local' && localData.unchanged !== true) {
        artistMap = new Map()
        albumMap = new Map()
        const classified = classify(localData.locaFilesMetadata)
        localStore.setLibraryData(
            localData.dirTree,
            localData.locaFilesMetadata,
            classified,
            localData.revision || null,
        )
        libraryStore.clearExpandedFolders()
        // Cached snapshots are complete by construction. A live scan can be
        // partial when a removable/NAS root is temporarily unavailable or a
        // traversal budget is hit; keep the pending queue for a later complete
        // scan instead of restoring only a subset and consuming it.
        if (localData.complete !== false) restorePlaylistFromLibrary(localStore.localMusicList)
    }
    if (localData.truncated) {
        noticeOpen('音乐库过大，已达到安全扫描上限', 4)
    } else if (localData.complete === false) {
        noticeOpen('部分音乐目录暂不可用，已保留播放恢复状态', 4)
    } else if (isRefreshLocalFile.value) {
        noticeOpen('扫描完毕 共' + localData.count + '首', 3)
    }
    isRefreshLocalFile.value = false
})
