import { createApp } from 'vue'
import pinia from './store/pinia'
import './style.css'
import 'normalize.css'
import './assets/css/common.css'
import './assets/css/fonts.css'

let bootStage = 'renderer-entry'
let disposeWindowApi = null
let disposePlayerLifecycle = null

const preventContextMenu = (event) => event.preventDefault()
document.addEventListener('contextmenu', preventContextMenu)

function errorDetail(error) {
  return error instanceof Error
    ? `${error.name}: ${error.message}${error.stack ? `\n${error.stack}` : ''}`
    : String(error)
}

function reportFrontendError(error, source) {
  const detail = errorDetail(error)
  console.error(`[${source}]`, detail)
  window.windowApi?.reportFrontendError?.(source, detail)?.catch((reportError) => {
    console.error('[error reporter]', reportError)
  })
}

function renderFatalBootstrapError(error, source) {
  const root = document.getElementById('app')
  if (!root) return
  const detail = errorDetail(error)
  root.innerHTML = ''
  root.style.cssText = [
    'position:fixed',
    'inset:0',
    'box-sizing:border-box',
    'padding:48px',
    'overflow:auto',
    'background:rgb(222,235,239)',
    'color:#111',
    'font-family:monospace',
    'user-select:text',
    'z-index:2147483647',
  ].join(';')

  const title = document.createElement('div')
  title.textContent = 'Hydrogen Music 启动失败'
  title.style.cssText = 'font-size:20px;font-weight:700;margin-bottom:14px'

  const stage = document.createElement('div')
  stage.textContent = `阶段: ${source}`
  stage.style.cssText = 'font-size:13px;margin-bottom:12px'

  const message = document.createElement('pre')
  message.textContent = detail
  message.style.cssText = 'font-size:12px;line-height:1.55;white-space:pre-wrap;word-break:break-word'

  root.append(title, stage, message)
}

async function bootstrap() {
  bootStage = 'load-platform-api'
  const { installWindowApi } = await import('./platform/windowApi')

  bootStage = 'install-window-api'
  disposeWindowApi = installWindowApi()

  bootStage = 'load-renderer-modules'
  const [
    { default: App },
    { default: router },
    { init },
    { default: lazy },
    { initializePlayerLifecycle },
  ] = await Promise.all([
    import('./App.vue'),
    import('./router/router.js'),
    import('./utils/initApp'),
    import('./utils/lazy'),
    import('./utils/player'),
  ])

  bootStage = 'create-vue-app'
  const app = createApp(App)
  app.config.errorHandler = (error, instance, info) => {
    reportFrontendError(error, `Vue: ${info}`)
  }
  app.use(pinia)
  app.use(router)
  app.directive('lazy', lazy)

  bootStage = 'mount-vue-app'
  app.mount('#app')

  bootStage = 'initialize-player-lifecycle'
  disposePlayerLifecycle = initializePlayerLifecycle()

  bootStage = 'initialize-persistent-state'
  init()
  bootStage = 'ready'
}

const handleWindowError = (event) => reportFrontendError(event.error || event.message, 'window.error')
const handleUnhandledRejection = (event) => reportFrontendError(event.reason, 'unhandledrejection')
window.addEventListener('error', handleWindowError)
window.addEventListener('unhandledrejection', handleUnhandledRejection)

bootstrap().catch((error) => {
  const source = `bootstrap:${bootStage}`
  reportFrontendError(error, source)
  renderFatalBootstrapError(error, source)
})

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    document.removeEventListener('contextmenu', preventContextMenu)
    window.removeEventListener('error', handleWindowError)
    window.removeEventListener('unhandledrejection', handleUnhandledRejection)
    disposePlayerLifecycle?.()
    disposeWindowApi?.()
  })
}
