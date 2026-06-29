import { useCallback } from 'react'
import { usePeticion } from '../../../hooks/usePeticion.js'
import { eventosApi } from '../services/eventosApi.js'

// Lista de todos los eventos.
export function useEventos() {
  return usePeticion(useCallback(() => eventosApi.listar(), []))
}

// Un evento por id. Si no hay id (modo "crear"), no pide nada.
export function useEvento(id) {
  return usePeticion(useCallback(() => (id ? eventosApi.obtener(id) : Promise.resolve(null)), [id]))
}
