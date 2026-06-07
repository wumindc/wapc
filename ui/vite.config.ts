import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  base: './',
  plugins: [react(), tailwindcss()],
  // Tauri expects a fixed port in dev mode
  server: {
    port: 5173,
    strictPort: true,
    host: '0.0.0.0',
  },
  // Produce dist that Tauri can load
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  // Tauri env variable prevents Vite from opening the browser
  clearScreen: false,
})
