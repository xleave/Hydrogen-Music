import assert from 'node:assert/strict'
import test from 'node:test'
import { DialogQueue } from '../src/utils/dialogQueue.mjs'

test('dialogs resolve callbacks in FIFO order', async () => {
  const visible = []
  const results = []
  const queue = new DialogQueue({
    show: (header, text) => visible.push([header, text]),
    hide: () => visible.push(null),
  })

  queue.open('First', 'Delete first?', (confirmed) => results.push(['first', confirmed]))
  queue.open('Second', 'Delete second?', (confirmed) => results.push(['second', confirmed]))

  assert.deepEqual(visible, [['First', 'Delete first?']])
  queue.resolve(true)
  await Promise.resolve()
  assert.deepEqual(visible, [['First', 'Delete first?'], null, ['Second', 'Delete second?']])
  queue.resolve(false)
  await Promise.resolve()
  assert.deepEqual(results, [['first', true], ['second', false]])
})

test('a dialog without a callback closes safely', async () => {
  let hidden = 0
  const queue = new DialogQueue({
    show: () => {},
    hide: () => { hidden += 1 },
  })

  queue.open('Notice', 'No callback')
  queue.resolve(true)
  await Promise.resolve()

  assert.equal(hidden, 1)
})

test('the scheduler is called without the dialog queue as its receiver', async () => {
  let callbackResult = null
  function receiverSensitiveSchedule(task) {
    assert.equal(this, undefined)
    queueMicrotask(task)
  }
  const queue = new DialogQueue({
    show: () => {},
    hide: () => {},
    schedule: receiverSensitiveSchedule,
  })

  queue.open('Enable blur', 'Continue?', (confirmed) => { callbackResult = confirmed })
  queue.resolve(true)
  await Promise.resolve()

  assert.equal(callbackResult, true)
})
