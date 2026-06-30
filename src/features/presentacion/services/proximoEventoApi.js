import { api } from '../../../lib/api.js'

// Próximo evento para la landing pública: sin login, sin datos sensibles
// (aforo, ventas). El backend devuelve null si no hay ninguno programado.
export const proximoEventoApi = {
  obtener: () => api.get('/publico/proximo-evento', { auth: false }),
}

// URL pública del afiche del evento a partir del nombre de archivo guardado.
export const urlImagenEvento = (nombre) => (nombre ? `/media/${nombre}` : null)
