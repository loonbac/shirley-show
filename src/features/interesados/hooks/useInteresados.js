import { useCallback } from 'react'
import { usePeticion } from '../../../hooks/usePeticion.js'
import { interesadosApi } from '../services/interesadosApi.js'

// Lista de interesados (fase 1) de un evento. `version` fuerza recarga desde fuera.
export function useInteresados(eventoId, version = 0) {
  const fn = useCallback(
    () => interesadosApi.listar(eventoId),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [eventoId, version],
  )
  return usePeticion(fn)
}
