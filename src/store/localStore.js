import { markRaw } from 'vue'
import { defineStore } from "pinia";

function asRaw(value) {
    return value == null ? value : markRaw(value)
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
            this.localDirectoryTree = asRaw(dirTree)
            this.localMusicList = asRaw(filesMetadata)
            this.localMusicClassify = asRaw(classifyData)
        },
        getSongs(arr, target = []) {
            for (const song of arr || []) {
                if (song.children) this.getSongs(song.children, target)
                else target.push(song)
            }
            return target
        },
        getFolderSongs(arr, folderId) {
            for (const item of arr || []) {
              if(item.id === folderId || item.dirPath === folderId || item.name === folderId) {
                this.currentSelectedInfo = {
                    id: item.id || item.dirPath,
                    name: item.name,
                    dirPath: item.dirPath
                }
                this.currentSelectedSongs = asRaw(this.getSongs(item.children, []))
                return true
              }
              if(item.children && this.getFolderSongs(item.children, folderId)) return true
            }
            return false
        },
        async getImgBase64(fileUrl) {
            return await windowApi.getLocalMusicImage(fileUrl)
        },
        updateLocalMusicDetail(type, query, id) {
            const requestId = ++this.detailRequestId
            this.currentType = type
            this.currentSelectedFilePicUrl = null
            if(type === 'localFiles') {
                if(query?.type === 'local')
                    this.getFolderSongs(this.localMusicList, query.id || query.name)
            }
            if(type === 'localAlbum') {
                const index = (this.localMusicClassify?.albums || []).findIndex((item) => item.id === id)
                if (index === -1) return false
                const album = this.localMusicClassify.albums[index]
                this.currentSelectedInfo = {
                    id: album.id,
                    name: album.name,
                    albumArtist: album.albumArtist,
                }
                this.currentSelectedSongs = asRaw(album.songs)
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        if (requestId === this.detailRequestId) this.currentSelectedFilePicUrl = res
                    }).catch((error) => console.error('[local cover]', error))
            }
            if(type === 'localArtist') {
                const index = (this.localMusicClassify?.artists || []).findIndex((item) => item.id === id)
                if (index === -1) return false
                const artist = this.localMusicClassify.artists[index]
                this.currentSelectedInfo = {
                    id: artist.id,
                    name: artist.name
                }
                this.currentSelectedSongs = asRaw(artist.songs)
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        if (requestId === this.detailRequestId) this.currentSelectedFilePicUrl = res
                    }).catch((error) => console.error('[local cover]', error))
            }
            return true
        }
    },
})
