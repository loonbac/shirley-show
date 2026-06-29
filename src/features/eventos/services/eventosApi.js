import { api } from '../../../lib/api.js'

// Acceso a datos de la feature de eventos.
export const eventosApi = {
  listar: () => api.get('/eventos'),
  obtener: (id) => api.get(`/eventos/${id}`),
  crear: (datos) => api.post('/eventos', datos),
  actualizar: (id, datos) => api.put(`/eventos/${id}`, datos),
  resumen: (id) => api.get(`/eventos/${id}/resumen`),
}
