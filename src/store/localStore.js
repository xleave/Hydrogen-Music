import { defineStore } from "pinia";

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

            quitApp: null,
        }
    },
    actions: {
        getSongs(arr) {
            for (const song of arr || []) {
                if (song.children) this.getSongs(song.children)
                else this.currentSelectedSongs.push(song)
            }
        },
        getFolderSongs(arr, folderName) {
            for (const item of arr || []) {
              if(item.name === folderName) {
                this.currentSelectedInfo = {
                    name: item.name,
                    dirPath: item.dirPath
                }
                this.currentSelectedSongs = []
                this.getSongs(item.children)
                return true
              }
              if(item.children && this.getFolderSongs(item.children, folderName)) return true
            }
            return false
        },
        async getImgBase64(fileUrl) {
            return await windowApi.getLocalMusicImage(fileUrl)
        },
        updateLocalMusicDetail(type, query, id) {
            this.currentType = type
            this.currentSelectedFilePicUrl = null
            if(type === 'localFiles') {
                if(query.type === 'local')
                    this.getFolderSongs(this.localMusicList, query.name)
            }
            if(type === 'localAlbum') {
                const index = (this.localMusicClassify?.albums || []).findIndex((item) => item.id === id)
                if (index === -1) return false
                const album = this.localMusicClassify.albums[index]
                this.currentSelectedInfo = {
                    id: album.id,
                    name: album.name
                }
                this.currentSelectedSongs = album.songs
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        this.currentSelectedFilePicUrl = res
                    })
            }
            if(type === 'localArtist') {
                const index = (this.localMusicClassify?.artists || []).findIndex((item) => item.id === id)
                if (index === -1) return false
                const artist = this.localMusicClassify.artists[index]
                this.currentSelectedInfo = {
                    id: artist.id,
                    name: artist.name
                }
                this.currentSelectedSongs = artist.songs
                if(this.currentSelectedSongs?.length)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        this.currentSelectedFilePicUrl = res
                    })
            }
            return true
        }
    },
})
