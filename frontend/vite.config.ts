import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'
import UnoCSS from 'unocss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), UnoCSS()],
  css: {
    modules: {
      generateScopedName: "css-[hash:6]",
    },
  },
  build: {
    rollupOptions: {
      output: {
        entryFileNames: "_components/js/[hash].js",
        chunkFileNames: "_components/js/[hash].js",
        manualChunks(id: string) {
          const match = id.match(/\/src\/components\/([^/]+)\//)
          if (match) return match[1]
        },
        assetFileNames: (assetInfo: { name?: string }) => {
          if (assetInfo.name && assetInfo.name.endsWith(".css")) {
            return "_components/css/[name]-[hash][extname]";
          }
          return "_assets/[name]-[hash][extname]";
        }
      }
    }
  }
})
