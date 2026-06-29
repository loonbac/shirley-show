import './Badge.css'

// Etiqueta de estado de color. tono: 'exito' | 'alerta' | 'error' | 'neutro' | 'primario'.
export function Badge({ tono = 'neutro', children }) {
  return <span className={`badge badge--${tono}`}>{children}</span>
}

// Mapea estados del dominio a un tono de color, para no repetir lógica en cada pantalla.
export const tonoPago = (estado) => (estado === 'Pagado' ? 'exito' : 'alerta')
export const tonoCheckin = (estado) => (estado === 'Usada' ? 'primario' : 'neutro')
export const tonoEvento = (estado) => (estado === 'Finalizado' ? 'neutro' : 'exito')
