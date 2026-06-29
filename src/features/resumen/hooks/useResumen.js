import { useCallback } from 'react'
import { usePeticion } from '../../../hooks/usePeticion.js'
import { resumenApi } from '../services/resumenApi.js'

// Resumen de un evento. `version` fuerza recarga cuando cambian las entradas.
export function useResumen(eventoId, version = 0) {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return usePeticion(useCallback(() => resumenApi.obtener(eventoId), [eventoId, version]))
}
