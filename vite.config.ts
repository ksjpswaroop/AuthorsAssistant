import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Tauri expects a fixed port and ignores the vite dev server's own HMR host.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    target: 'es2022',
    sourcemap: true,
    rollupOptions: {
      // Two entries: the app, and the splash window Tauri shows while it boots.
      input: {
        main: 'index.html',
        splash: 'splash.html',
      },
    },
  },
})
