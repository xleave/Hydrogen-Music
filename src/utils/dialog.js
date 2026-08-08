import { useOtherStore } from '../store/otherStore';
import { storeToRefs } from 'pinia';
const otherStore = useOtherStore()
const { dialogShow, dialogHeader, dialogText, noticeShow, noticeText, noticeOutAnimation } = storeToRefs(otherStore)

const dialogQueue = []
let activeDialog = null

function showNextDialog() {
    if (activeDialog || dialogQueue.length === 0) return
    activeDialog = dialogQueue.shift()
    dialogSetter(activeDialog.header, activeDialog.text)
    dialogShow.value = true
}

export function dialogOpen(header, text, callback) {
    dialogQueue.push({ header, text, callback: typeof callback === 'function' ? callback : null })
    showNextDialog()
}

export function dialogClose() {
    dialogShow.value = false
    activeDialog = null
    dialogClear()
    queueMicrotask(showNextDialog)
}

export function dialogSetter(header, text) {
    dialogHeader.value = header
    dialogText.value = text
}

export function dialogClear() {
    dialogHeader.value = null
    dialogText.value = null
}

function resolveDialog(result) {
    const callback = activeDialog?.callback
    dialogClose()
    callback?.(result)
}

export function dialogCancel() {
    resolveDialog(false)
}

export function dialogConfirm() {
    resolveDialog(true)
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
