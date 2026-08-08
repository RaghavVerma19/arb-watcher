import { defineConfig, loadEnv } from 'vite'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const backend = env.ARB_BACKEND ?? 'http://127.0.0.1:8080'
  const wsTarget = backend.replace(/^http/, 'ws')
  return {
    plugins: [tailwindcss()],
    server: {
      proxy: {
        '/api': backend,
        '/ws': {
          target: wsTarget,
          ws: true,
        },
      },
    },
  }
})
