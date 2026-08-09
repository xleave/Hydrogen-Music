import { defineStore } from 'pinia'

export const useOtherStore = defineStore('otherStore', {
  state: () => ({
    contextMenuShow: false,
    selectedItem: null,
    selectedCollectionId: null,
    dialogShow: false,
    dialogHeader: null,
    dialogText: null,
    noticeShow: false,
    noticeText: null,
    noticeOutAnimation: false,
  }),
})
