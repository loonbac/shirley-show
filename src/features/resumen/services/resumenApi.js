import { api } from '../../../lib/api.js'

// Acceso a datos del resumen económico y de aforo de un evento.
export const resumenApi = {
  obtener: (eventoId) => api.get(`/eventos/${eventoId}/resumen`),
}
