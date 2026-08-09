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
let trackById = new Map()
let trackSearchKeyByObject = new WeakMap()
let trackModifiedOrderByList = new WeakMap()

function trackSearchKey(track) {
    if (!track || typeof track !== 'object') return ''
    const cached = trackSearchKeyByObject.get(track)
    if (cached !== undefined) return cached
    const common = track.common || {}
    const key = [
        common.title,
        common.localTitle,
        common.album,
        common.albumartist,
        ...(common.artists || []),
    ]
        .filter(Boolean)
        .join('\n')
        .toLocaleLowerCase()
    trackSearchKeyByObject.set(track, key)
    return key
}

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
    trackById = new Map()
    trackSearchKeyByObject = new WeakMap()

    indexFolderTree(filesMetadata)
    for (const song of flattenedSongs) {
        if (song?.id) trackById.set(song.id, song)
    }
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
            libraryRevision: null,

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
        setLibraryData(dirTree, filesMetadata, classifyData, revision = null) {
            const selectedType = this.currentType
            const selectedId = this.currentSelectedInfo?.id
                || this.currentSelectedInfo?.dirPath
                || this.currentSelectedFile?.id
                || this.currentSelectedFile?.dirPath
            const rawDirTree = asRaw(dirTree)
            const rawFilesMetadata = asRaw(filesMetadata)
            const rawClassifyData = asRaw(classifyData)
            rebuildLibraryIndexes(rawFilesMetadata, rawClassifyData)
            this.localDirectoryTree = rawDirTree
            this.localMusicList = rawFilesMetadata
            this.localMusicClassify = rawClassifyData
            this.libraryRevision = revision

            if (selectedType && selectedId) {
                const query = selectedType === 'localFiles'
                    ? { id: selectedId, type: 'local' }
                    : null
                this.updateLocalMusicDetail(selectedType, query, selectedId)
            }
        },
        getSongs(arr, target = []) {
            for (const song of arr || []) {
                if (song.children) this.getSongs(song.children, target)
                else target.push(song)
            }
            return target
        },
        resolveTrackIds(trackIds) {
            return asRaw((trackIds || []).map((id) => trackById.get(id)).filter(Boolean))
        },
        filterTracks(tracks, query) {
            const keyword = String(query || '').trim().toLocaleLowerCase()
            if (!keyword) return tracks || []
            return asRaw((tracks || []).filter((track) => trackSearchKey(track).includes(keyword)))
        },
        sortTracksByModified(tracks) {
            if (!Array.isArray(tracks) || tracks.length < 2) return tracks || []
            let sorted = trackModifiedOrderByList.get(tracks)
            if (!sorted) {
                sorted = asRaw([...tracks].sort(
                    (left, right) => (right.common?.modifiedAt ?? 0) - (left.common?.modifiedAt ?? 0),
                ))
                trackModifiedOrderByList.set(tracks, sorted)
            }
            return sorted
        },
        getFolderSongs(arr, folderId) {
            const item = folderById.get(folderId)
            const songs = songsForRange(folderRangeById.get(folderId))
            if (item && songs) {
                this.currentSelectedFile = item
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
                this.currentSelectedFile = candidate
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
        clearSelectedDetail() {
            this.currentSelectedFile = {name: null}
            this.currentSelectedInfo = null
            this.currentSelectedSongs = null
            this.currentSelectedFilePicUrl = null
        },
        updateLocalMusicDetail(type, query, id) {
            const requestId = ++this.detailRequestId
            this.currentType = type
            this.currentSelectedFilePicUrl = null
            if(type === 'localFiles') {
                const found = query?.type === 'local'
                    && this.getFolderSongs(this.localMusicList, query.id || query.name)
                if (!found) this.clearSelectedDetail()
                return Boolean(found)
            }
            if(type === 'localAlbum') {
                const album = albumById.get(id) || (this.localMusicClassify?.albums || []).find(
                    (item) => item.id === id || JSON.stringify([item.albumArtist, item.name]) === id,
                )
                if (!album) {
                    this.clearSelectedDetail()
                    return false
                }
                this.currentSelectedFile = {name: null}
                this.currentSelectedInfo = {
                    id: album.id,
                    name: album.name,
                    albumArtist: album.albumArtist,
                }
                this.currentSelectedSongs = this.resolveTrackIds(album.trackIds)
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        if (requestId === this.detailRequestId) this.currentSelectedFilePicUrl = res
                    }).catch((error) => console.error('[local cover]', error))
            }
            if(type === 'localArtist') {
                const artist = artistById.get(id) || (this.localMusicClassify?.artists || []).find(
                    (item) => item.id === id || item.name === id,
                )
                if (!artist) {
                    this.clearSelectedDetail()
                    return false
                }
                this.currentSelectedFile = {name: null}
                this.currentSelectedInfo = {
                    id: artist.id,
                    name: artist.name
                }
                this.currentSelectedSongs = this.resolveTrackIds(artist.trackIds)
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        if (requestId === this.detailRequestId) this.currentSelectedFilePicUrl = res
                    }).catch((error) => console.error('[local cover]', error))
            }
            return type === 'localAlbum' || type === 'localArtist'
        }
    },
})
