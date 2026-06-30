import { api } from '../../../lib/api.js'

// Acceso a datos de los gastos de un evento.
export const gastosApi = {
  listar: (eventoId) => api.get(`/eventos/${eventoId}/gastos`),
  crear: (eventoId, datos) => api.post(`/eventos/${eventoId}/gastos`, datos),
  actualizar: (id, datos) => api.put(`/gastos/${id}`, datos),
  eliminar: (id) => api.del(`/gastos/${id}`),
}
