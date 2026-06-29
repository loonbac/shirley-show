import { useState, useRef, useCallback } from 'react'
import { checkinApi } from '../services/checkinApi.js'

// Lógica del check-in: valida un código escaneado y guarda el último resultado.
// Evita procesar el mismo código repetidamente mientras la cámara lo sigue viendo.
export function useCheckin(eventoId) {
  const [resultado, setResultado] = useState(null)
  const [procesando, setProcesando] = useState(false)
  const ultimoRef = useRef({ codigo: null, t: 0 })

  const validar = useCallback(
    async (codigo) => {
      const ahora = Date.now()
      const ult = ultimoRef.current
      // Ignora el mismo código si se reescanea en menos de 3 segundos.
      if (codigo === ult.codigo && ahora - ult.t < 3000) return
      ultimoRef.current = { codigo, t: ahora }

      setProcesando(true)
      try {
        const res = await checkinApi.validar(eventoId, codigo)
        setResultado({ ...res, en: ahora })
      } catch {
        setResultado({ resultado: 'invalida', en: ahora })
      } finally {
        setProcesando(false)
      }
    },
    [eventoId],
  )

  return { resultado, procesando, validar }
}
