import { defineStore } from 'pinia'

export const useLibraryStore = defineStore('libraryStore', {
  state: () => ({
    listType1: 3,
    listType2: 0,
    libraryChangeAnimation: false,
    expandedFolderIds: {},
  }),
  actions: {
    folderExpanded(id) {
      return Boolean(id && this.expandedFolderIds[id])
    },
    toggleFolder(id) {
      if (!id) return
      const next = { ...this.expandedFolderIds }
      if (next[id]) delete next[id]
      else next[id] = true
      this.expandedFolderIds = next
    },
    clearExpandedFolders() {
      this.expandedFolderIds = {}
    },
  },
})
