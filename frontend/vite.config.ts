import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'
import UnoCSS from 'unocss/vite'
import wasm from 'vite-plugin-wasm'

export default defineConfig({
  plugins: [react(), UnoCSS(), wasm()],
  worker: {
    format: 'es',
    plugins: () => [wasm()],
  },
  css: {
    modules: {
      generateScopedName: 'css-[hash:6]',
    },
  },
  build: {
    // Native top-level await — required by vite-plugin-wasm without the
    // vite-plugin-top-level-await transform
    target: 'esnext',
    rollupOptions: {
      output: {
        entryFileNames: '_components/js/[hash].js',
        chunkFileNames: '_components/js/[hash].js',
        manualChunks(id: string) {
          const match = id.match(/\/src\/components\/([^/]+)\//)
          if (match) return match[1]
        },
        assetFileNames: (assetInfo: { name?: string }) => {
          if (assetInfo.name?.endsWith('.css')) {
            return '_components/css/[name]-[hash][extname]'
          }
          return '_assets/[name]-[hash][extname]'
        },
      },
    },
  },
})
