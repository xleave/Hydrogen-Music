export class DialogQueue {
  constructor({ show, hide, schedule = queueMicrotask }) {
    this.show = show
    this.hide = hide
    // WebKitGTK requires native Window methods such as queueMicrotask to be
    // called without rebinding their receiver to the queue instance.
    this.schedule = (task) => schedule(task)
    this.pending = []
    this.active = null
  }

  open(header, text, callback) {
    this.pending.push({
      header,
      text,
      callback: typeof callback === 'function' ? callback : null,
    })
    this.showNext()
  }

  showNext() {
    if (this.active || this.pending.length === 0) return
    this.active = this.pending.shift()
    this.show(this.active.header, this.active.text)
  }

  close() {
    if (!this.active) return
    this.active = null
    this.hide()
    this.schedule(() => this.showNext())
  }

  resolve(result) {
    const callback = this.active?.callback
    this.close()
    callback?.(result)
  }
}
