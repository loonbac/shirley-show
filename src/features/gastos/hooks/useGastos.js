import { useCallback } from 'react'
import { usePeticion } from '../../../hooks/usePeticion.js'
import { gastosApi } from '../services/gastosApi.js'

// Lista de gastos de un evento. `version` fuerza recarga desde fuera.
export function useGastos(eventoId, version = 0) {
  const fn = useCallback(
    () => gastosApi.listar(eventoId),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [eventoId, version],
  )
  return usePeticion(fn)
}
