import { createApp } from 'vue'
import pinia from './store/pinia'
import './style.css'
import 'normalize.css'
import './assets/css/common.css'
import './assets/css/fonts.css'
import { installWindowApi } from './platform/windowApi'

installWindowApi()

// 禁用 WebView 默认右键菜单（浏览器右键）
document.addEventListener('contextmenu', (e) => e.preventDefault())

async function bootstrap() {
  const [{ default: App }, { default: router }, { init }, { default: lazy }] = await Promise.all([
    import('./App.vue'),
    import('./router/router.js'),
    import('./utils/initApp'),
    import('./utils/lazy'),
  ])
  const app = createApp(App)
  app.use(router)
  app.use(pinia)
  app.directive('lazy', lazy)
  app.mount('#app')
  init()
}

bootstrap()
