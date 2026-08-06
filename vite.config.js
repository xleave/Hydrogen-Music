import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import {resolve} from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  base:'./',
  manifest:true,
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
      'axios': resolve(__dirname, './src/platform/noNetwork.js')
    }
  },
  optimizeDeps: {
    exclude: []
  }
})
