import { api } from '../../../lib/api.js'

// Acceso a datos de la fase 1 (interesados / invitación).
export const interesadosApi = {
  listar: (eventoId) => api.get(`/eventos/${eventoId}/interesados`),
  crear: (eventoId, datos) => api.post(`/eventos/${eventoId}/interesados`, datos),
  actualizar: (id, datos) => api.put(`/interesados/${id}`, datos),
  confirmar: (id) => api.post(`/interesados/${id}/confirmar`),
  eliminar: (id) => api.del(`/interesados/${id}`),
}
