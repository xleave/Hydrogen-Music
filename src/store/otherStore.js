import { defineStore } from 'pinia'

export const useOtherStore = defineStore('otherStore', {
  state: () => ({
    contextMenuShow: false,
    menuTree: [
      { id: 8, name: '播放' },
      { id: 9, name: '下一首播放' },
      { id: 10, name: '打开本地文件夹' },
    ],
    selectedItem: null,
    dialogShow: false,
    dialogHeader: null,
    dialogText: null,
    noticeShow: false,
    noticeText: null,
    noticeOutAnimation: false,
  }),
})
