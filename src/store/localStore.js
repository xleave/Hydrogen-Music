import { defineStore } from "pinia";

export const useLocalStore = defineStore('localStore', {
    state: () => {
        return {
            localFolderSettings: [],
            localMusicFolder: null,
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
            arr.forEach(song => {
              if(song.children) this.getSongs(song.children)
              else {
                this.currentSelectedSongs.push(song)
              }
            })
        },
        getFolderSongs(arr, folderName) {
            arr.forEach(item => {
              if(item.name == folderName) {
                this.currentSelectedInfo = {
                    name: item.name,
                    dirPath: item.dirPath
                }
                this.currentSelectedSongs = []
                this.getSongs(item.children)
                return
              } else if(item.children) this.getFolderSongs(item.children, folderName)
        
            });
        },
        async getImgBase64(fileUrl) {
            return await windowApi.getLocalMusicImage(fileUrl)
        },
        updateLocalMusicDetail(type, query, id) {
            this.currentType = type
            if(type == 'localFiles') {
                if(query.type == 'local')
                    this.getFolderSongs(this.localMusicList, query.name)
            }
            if(type == 'localAlbum') {
                const index = (this.localMusicClassify.albums || []).findIndex((item) => item.id == id)
                this.currentSelectedInfo = {
                    id: this.localMusicClassify.albums[index].id,
                    name: this.localMusicClassify.albums[index].name
                }
                this.currentSelectedSongs = this.localMusicClassify.albums[index].songs
                if(this.currentSelectedSongs)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        this.currentSelectedFilePicUrl = res
                    })
            }
            if(type == 'localArtist') {
                const index = (this.localMusicClassify.artists || []).findIndex((item) => item.id == id)
                this.currentSelectedInfo = {
                    id: this.localMusicClassify.artists[index].id,
                    name: this.localMusicClassify.artists[index].name
                }
                this.currentSelectedSongs = this.localMusicClassify.artists[index].songs
                if(this.currentSelectedSongs)
                    this.getImgBase64(this.currentSelectedSongs[0].common.fileUrl).then(res => {
                        this.currentSelectedFilePicUrl = res
                    })
            }
        }
    },
})
