import { useState, useEffect, useCallback } from 'react'

// Hook global reutilizable para pedir datos: maneja carga, error y recarga.
// `fn` debe ser estable (envuélvela en useCallback en quien lo use).
export function usePeticion(fn) {
  const [datos, setDatos] = useState(null)
  const [cargando, setCargando] = useState(true)
  const [error, setError] = useState('')

  const ejecutar = useCallback(async () => {
    setCargando(true)
    setError('')
    try {
      setDatos(await fn())
    } catch (e) {
      setError(e.message)
    } finally {
      setCargando(false)
    }
  }, [fn])

  useEffect(() => {
    ejecutar()
  }, [ejecutar])

  return { datos, cargando, error, recargar: ejecutar }
}
