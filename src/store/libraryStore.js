import { defineStore } from 'pinia'

export const useLibraryStore = defineStore('libraryStore', {
  state: () => ({
    listType1: 3,
    listType2: 0,
    libraryChangeAnimation: false,
  }),
})
