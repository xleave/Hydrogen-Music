import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  base: './',
  manifest: true,
  // Hydrogen Music 的开发服务器只服务本机 Tauri WebView；明确绑定 loopback，
  // 避免旧版 Vite dev-server 文件系统问题被网络暴露放大。
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    fs: { strict: true },
  },
  preview: {
    host: '127.0.0.1',
  },
  resolve: {
    alias: [
      { find: '@', replacement: resolve(__dirname, './src') },
      { find: 'axios', replacement: resolve(__dirname, './src/platform/noNetwork.js') },
      // All Hydrogen lists use a fixed row height. Keep the existing import
      // surface while replacing the generic per-row transform implementation
      // with our WebKitGTK-friendly fixed-row scroller.
      { find: /^vue-virtual-scroller$/, replacement: resolve(__dirname, './src/components/FixedVirtualScroller.js') },
    ]
  },
  optimizeDeps: {
    exclude: []
  }
})
