import { api } from '../../../lib/api.js'

// Acceso a datos de la feature de entradas.
export const entradasApi = {
  listar: (eventoId, filtros = {}) => {
    const qs = new URLSearchParams()
    if (filtros.estado_pago) qs.set('estado_pago', filtros.estado_pago)
    if (filtros.q) qs.set('q', filtros.q)
    const cola = qs.toString() ? `?${qs}` : ''
    return api.get(`/eventos/${eventoId}/entradas${cola}`)
  },
  registrarVenta: (eventoId, datos) => api.post(`/eventos/${eventoId}/entradas`, datos),
  confirmarPago: (id) => api.post(`/entradas/${id}/confirmar-pago`),
  // Vista pública (sin login) por código único, para compartir por WhatsApp.
  porCodigo: (codigo) => api.get(`/publico/entrada/${codigo}`, { auth: false }),
}
