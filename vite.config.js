import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// El frontend (Vite) corre en :5173 y el API (Express) en :3001.
// Todas las llamadas a /api se redirigen al backend en desarrollo.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    host: true, // accesible desde el celular en la red local
    proxy: {
      '/api': 'http://localhost:3001',
    },
  },
})
