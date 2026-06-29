// Cliente HTTP único para hablar con el API. Centraliza token, JSON y errores.
const CLAVE_TOKEN = 'shirley_token'

export const getToken = () => localStorage.getItem(CLAVE_TOKEN)
export const setToken = (t) => localStorage.setItem(CLAVE_TOKEN, t)
export const limpiarToken = () => localStorage.removeItem(CLAVE_TOKEN)

async function pedir(metodo, ruta, { body, auth = true } = {}) {
  const headers = { 'Content-Type': 'application/json' }
  if (auth) {
    const token = getToken()
    if (token) headers.Authorization = `Bearer ${token}`
  }
  const res = await fetch(`/api${ruta}`, {
    method: metodo,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  })
  const datos = res.status === 204 ? null : await res.json().catch(() => null)
  if (!res.ok) {
    if (res.status === 401) limpiarToken()
    throw new Error(datos?.error || 'Ocurrió un error')
  }
  return datos
}

export const api = {
  get: (ruta, opts) => pedir('GET', ruta, opts),
  post: (ruta, body, opts) => pedir('POST', ruta, { ...opts, body }),
  put: (ruta, body, opts) => pedir('PUT', ruta, { ...opts, body }),
  del: (ruta, opts) => pedir('DELETE', ruta, opts),
}
