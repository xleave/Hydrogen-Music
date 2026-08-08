import { createApp } from 'vue'
import pinia from './store/pinia'
import './style.css'
import 'normalize.css'
import './assets/css/common.css'
import './assets/css/fonts.css'
import { installWindowApi } from './platform/windowApi'
import { initializePlayerLifecycle } from './utils/player'

const disposeWindowApi = installWindowApi()
let disposePlayerLifecycle = null

const preventContextMenu = (event) => event.preventDefault()
document.addEventListener('contextmenu', preventContextMenu)

function reportFrontendError(error, source) {
  const detail = error instanceof Error
    ? `${error.name}: ${error.message}${error.stack ? `\n${error.stack}` : ''}`
    : String(error)
  console.error(`[${source}]`, detail)
  windowApi?.reportFrontendError?.(source, detail)?.catch((reportError) => {
    console.error('[error reporter]', reportError)
  })
}

async function bootstrap() {
  const [{ default: App }, { default: router }, { init }, { default: lazy }] = await Promise.all([
    import('./App.vue'),
    import('./router/router.js'),
    import('./utils/initApp'),
    import('./utils/lazy'),
  ])
  const app = createApp(App)
  app.config.errorHandler = (error, instance, info) => {
    reportFrontendError(error, `Vue: ${info}`)
  }
  app.use(pinia)
  app.use(router)
  app.directive('lazy', lazy)
  app.mount('#app')
  disposePlayerLifecycle = initializePlayerLifecycle()
  init()
}

const handleWindowError = (event) => reportFrontendError(event.error || event.message, 'window.error')
const handleUnhandledRejection = (event) => reportFrontendError(event.reason, 'unhandledrejection')
window.addEventListener('error', handleWindowError)
window.addEventListener('unhandledrejection', handleUnhandledRejection)

bootstrap().catch((error) => reportFrontendError(error, 'bootstrap'))

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    document.removeEventListener('contextmenu', preventContextMenu)
    window.removeEventListener('error', handleWindowError)
    window.removeEventListener('unhandledrejection', handleUnhandledRejection)
    disposePlayerLifecycle?.()
    disposeWindowApi()
  })
}
