import { useOtherStore } from '../store/otherStore';
import { storeToRefs } from 'pinia';
import { DialogQueue } from './dialogQueue.mjs'
const otherStore =  useOtherStore()
const { dialogShow, dialogHeader, dialogText, noticeShow, noticeText, noticeOutAnimation } = storeToRefs(otherStore)

const dialogQueue = new DialogQueue({
    show: (header, text) => {
        dialogSetter(header, text)
        dialogShow.value = true
    },
    hide: () => {
        dialogShow.value = false
        dialogClear()
    },
})

export function dialogOpen(header, text, callback) {
    dialogQueue.open(header, text, callback)
}
export function dialogClose() {
    dialogQueue.close()
}
export function dialogSetter(header, text) {
    dialogHeader.value = header
    dialogText.value = text
}
export function dialogClear() {
    dialogHeader.value = null
    dialogText.value = null
}
export function dialogCancel() {
    dialogQueue.resolve(false)
}
export function dialogConfirm() {
    dialogQueue.resolve(true)
}

let noticeTimer1 = null
let noticeTimer2 = null
export function noticeOpen(text, duration) {
    noticeShow.value = false
    noticeOutAnimation.value = false
    clearTimeout(noticeTimer1)
    clearTimeout(noticeTimer2)
    noticeShow.value = true
    noticeText.value = text
    
    noticeTimer1 = setTimeout(() => {
        noticeOutAnimation.value = true
        clearTimeout(noticeTimer1)
        noticeTimer2 = setTimeout(() => {
            noticeShow.value = false
            noticeOutAnimation.value = false
            clearTimeout(noticeTimer2)
        }, 300);
    }, duration * 1000);
}
