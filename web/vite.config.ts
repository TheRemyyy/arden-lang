import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import vike from 'vike/plugin'
import compression from 'vite-plugin-compression'
import { webfontDl } from 'vite-plugin-webfont-dl'

export default defineConfig({
  plugins: [
    vike(),
    react(),
    compression({ algorithm: 'gzip', ext: '.gz' }),
    webfontDl()
  ],
  test: {
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
  },
  build: {
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true,
      },
    },
    rollupOptions: {
      output: {
        manualChunks: undefined
      },
    },
  },
})
