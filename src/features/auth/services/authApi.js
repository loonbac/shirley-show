import { api } from '../../../lib/api.js'

// Acceso a datos de la feature de autenticación.
export const authApi = {
  login: (usuario, password) => api.post('/auth/login', { usuario, password }, { auth: false }),
}
