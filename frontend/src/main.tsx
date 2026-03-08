import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import 'virtual:uno.css'
import './index.css'
import App from './App.tsx'

createRoot(document.getElementById('container')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
