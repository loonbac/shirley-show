import { useCallback } from 'react'
import { usePeticion } from '../../../hooks/usePeticion.js'
import { proximoEventoApi } from '../services/proximoEventoApi.js'

// Trae el próximo evento (o null si no hay ninguno) para mostrarlo en la landing.
export function useProximoEvento() {
  const pedir = useCallback(() => proximoEventoApi.obtener(), [])
  const { datos, cargando } = usePeticion(pedir)
  return { evento: datos, cargando }
}
