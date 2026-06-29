import { Navigate } from 'react-router-dom'
import { useSesion } from '../features/auth/index.js'

// Solo deja pasar a usuarios autenticados; el resto va al login.
export function RutaProtegida({ children }) {
  const { autenticado } = useSesion()
  if (!autenticado) return <Navigate to="/" replace />
  return children
}
