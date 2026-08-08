import { markRaw } from 'vue'
import { defineStore } from "pinia";

function asRaw(value) {
    return value == null ? value : markRaw(value)
}

// Library metadata is immutable between scans. Navigation indexes are kept
// outside Pinia so the large metadata graph never enters Vue deep reactivity.
let folderById = new Map()
let folderRangeById = new Map()
let folderSongsByRange = new WeakMap()
let flattenedSongs = []
let albumById = new Map()
let artistById = new Map()

function addFolderAlias(map, key, value) {
    if (key != null && key !== '' && !map.has(key)) map.set(key, value)
}

function indexFolderTree(nodes) {
    for (const item of nodes || []) {
        if (Array.isArray(item.children)) {
            const start = flattenedSongs.length
            indexFolderTree(item.children)
            const range = { start, end: flattenedSongs.length }
            const primaryId = item.id || item.dirPath || item.name
            addFolderAlias(folderById, primaryId, item)
            addFolderAlias(folderById, item.dirPath, item)
            // Name is retained only as a legacy route alias and never replaces
            // a stable id/path mapping when duplicate folder names exist.
            addFolderAlias(folderById, item.name, item)
            addFolderAlias(folderRangeById, primaryId, range)
            addFolderAlias(folderRangeById, item.dirPath, range)
            addFolderAlias(folderRangeById, item.name, range)
        } else {
            flattenedSongs.push(item)
        }
    }
}

function rebuildLibraryIndexes(filesMetadata, classifyData) {
    folderById = new Map()
    folderRangeById = new Map()
    folderSongsByRange = new WeakMap()
    flattenedSongs = []
    albumById = new Map()
    artistById = new Map()

    indexFolderTree(filesMetadata)
    for (const album of classifyData?.albums || []) albumById.set(album.id, album)
    for (const artist of classifyData?.artists || []) artistById.set(artist.id, artist)
}

function songsForRange(range) {
    if (!range) return null
    let songs = folderSongsByRange.get(range)
    if (!songs) {
        songs = asRaw(flattenedSongs.slice(range.start, range.end))
        folderSongsByRange.set(range, songs)
    }
    return songs
}

export const useLocalStore = defineStore('localStore', {
    state: () => {
        return {
            localFolderSettings: [],
            localDirectoryTree: null,
            localMusicList: null,
            localMusicClassify: null,

            currentSelectedFile: {name: null},

            currentType: null,
            currentSelectedInfo: null,
            currentSelectedSongs: null,
            currentSelectedFilePicUrl: null,
            isRefreshLocalFile: false,
            detailRequestId: 0,

            quitApp: null,
        }
    },
    actions: {
        setLibraryData(dirTree, filesMetadata, classifyData) {
            const rawDirTree = asRaw(dirTree)
            const rawFilesMetadata = asRaw(filesMetadata)
            const rawClassifyData = asRaw(classifyData)
            rebuildLibraryIndexes(rawFilesMetadata, rawClassifyData)
            this.localDirectoryTree = rawDirTree
            this.localMusicList = rawFilesMetadata
            this.localMusicClassify = rawClassifyData
        },
        getSongs(arr, target = []) {
            for (const song of arr || []) {
                if (song.children) this.getSongs(song.children, target)
                else target.push(song)
            }
            return target
        },
        getFolderSongs(arr, folderId) {
            const item = folderById.get(folderId)
            const songs = songsForRange(folderRangeById.get(folderId))
            if (item && songs) {
                this.currentSelectedInfo = {
                    id: item.id || item.dirPath,
                    name: item.name,
                    dirPath: item.dirPath
                }
                this.currentSelectedSongs = songs
                return true
            }

            // Compatibility fallback for state restored before the first
            // indexed snapshot has been installed. It should not run during
            // normal folder navigation after setLibraryData().
            for (const candidate of arr || []) {
              if(candidate.id === folderId || candidate.dirPath === folderId || candidate.name === folderId) {
                this.currentSelectedInfo = {
                    id: candidate.id || candidate.dirPath,
                    name: candidate.name,
                    dirPath: candidate.dirPath
                }
                this.currentSelectedSongs = asRaw(this.getSongs(candidate.children, []))
                return true
              }
              if(candidate.children && this.getFolderSongs(candidate.children, folderId)) return true
            }
            return false
        },
        async getImgBase64(fileUrl) {
            return await windowApi.getLocalMusicImage(fileUrl)
        },
        updateLocalMusicDetail(type, query, id) {
            const requestId = ++this.detailRequestId
            this.currentType = type
            if(type === 'localFiles') {
                this.currentSelectedFilePicUrl = null
                if(query?.type === 'local')
                    this.getFolderSongs(this.localMusicList, query.id || query.name)
            }
            if(type === 'localAlbum') {
                const album = albumById.get(id) || (this.localMusicClassify?.albums || []).find((item) => item.id === id)
                if (!album) return false
                this.currentSelectedInfo = {
                    id: album.id,
                    name: album.name,
                    albumArtist: album.albumArtist,
                }
                this.currentSelectedSongs = asRaw(album.songs)
                this.currentSelectedFilePicUrl = null
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        if (requestId === this.detailRequestId) this.currentSelectedFilePicUrl = res
                    }).catch((error) => console.error('[local cover]', error))
            }
            if(type === 'localArtist') {
                const artist = artistById.get(id) || (this.localMusicClassify?.artists || []).find((item) => item.id === id)
                if (!artist) return false
                this.currentSelectedInfo = {
                    id: artist.id,
                    name: artist.name
                }
                this.currentSelectedSongs = asRaw(artist.songs)
                this.currentSelectedFilePicUrl = null
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        if (requestId === this.detailRequestId) this.currentSelectedFilePicUrl = res
                    }).catch((error) => console.error('[local cover]', error))
            }
            return true
        }
    },
})
