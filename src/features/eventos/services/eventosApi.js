import { api, getToken } from '../../../lib/api.js'

// Acceso a datos de la feature de eventos.
export const eventosApi = {
  listar: () => api.get('/eventos'),
  obtener: (id) => api.get(`/eventos/${id}`),
  crear: (datos) => api.post('/eventos', datos),
  actualizar: (id, datos) => api.put(`/eventos/${id}`, datos),
  finalizar: (id) => api.post(`/eventos/${id}/finalizar`),
  reabrir: (id) => api.post(`/eventos/${id}/reabrir`),
  eliminar: (id) => api.del(`/eventos/${id}`),
  resumen: (id) => api.get(`/eventos/${id}/resumen`),

  // Sube el afiche (multipart). No usa el helper api porque manda FormData, no JSON.
  subirImagen: async (id, file) => {
    const fd = new FormData()
    fd.append('imagen', file)
    const res = await fetch(`/api/eventos/${id}/imagen`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${getToken()}` },
      body: fd,
    })
    const datos = await res.json().catch(() => null)
    if (!res.ok) throw new Error(datos?.error || 'No se pudo subir la imagen')
    return datos
  },
  quitarImagen: (id) => api.del(`/eventos/${id}/imagen`),
}

// URL pública del afiche de un evento a partir del nombre de archivo guardado.
export const urlImagen = (nombre) => (nombre ? `/media/${nombre}` : null)
