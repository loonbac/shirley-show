import { createContext, useContext, useState, useCallback } from 'react'
import { getToken, setToken, limpiarToken } from '../../../lib/api.js'
import { authApi } from '../services/authApi.js'

const CLAVE_USUARIO = 'shirley_usuario'
const SesionContext = createContext(null)

// Provee el estado de sesión a toda la app (token + usuario).
export function ProveedorSesion({ children }) {
  const [usuario, setUsuario] = useState(() => localStorage.getItem(CLAVE_USUARIO))
  const [autenticado, setAutenticado] = useState(() => Boolean(getToken()))

  const iniciarSesion = useCallback(async (u, p) => {
    const res = await authApi.login(u, p)
    setToken(res.token)
    localStorage.setItem(CLAVE_USUARIO, res.usuario)
    setUsuario(res.usuario)
    setAutenticado(true)
  }, [])

  const cerrarSesion = useCallback(() => {
    limpiarToken()
    localStorage.removeItem(CLAVE_USUARIO)
    setUsuario(null)
    setAutenticado(false)
  }, [])

  return (
    <SesionContext.Provider value={{ usuario, autenticado, iniciarSesion, cerrarSesion }}>
      {children}
    </SesionContext.Provider>
  )
}

export const useSesion = () => useContext(SesionContext)
