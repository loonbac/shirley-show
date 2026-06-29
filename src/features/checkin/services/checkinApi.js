import { api } from '../../../lib/api.js'

// Acceso a datos del check-in en puerta.
export const checkinApi = {
  validar: (eventoId, codigo) => api.post(`/eventos/${eventoId}/checkin`, { codigo }),
}
