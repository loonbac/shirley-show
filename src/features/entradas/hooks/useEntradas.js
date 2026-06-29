import { useCallback } from 'react'
import { usePeticion } from '../../../hooks/usePeticion.js'
import { entradasApi } from '../services/entradasApi.js'

// Lista de entradas de un evento, con filtros (estado de pago y búsqueda por nombre).
// `version` permite forzar recarga desde fuera cuando cambian los datos.
export function useEntradas(eventoId, filtros, version = 0) {
  const fn = useCallback(
    () => entradasApi.listar(eventoId, filtros),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [eventoId, filtros.estado_pago, filtros.q, version],
  )
  return usePeticion(fn)
}
