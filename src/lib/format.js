// Utilidades de formato reutilizables (moneda y fechas en español, Perú).

export const soles = (n) =>
  new Intl.NumberFormat('es-PE', { style: 'currency', currency: 'PEN' }).format(Number(n || 0))

export const fechaLarga = (iso) =>
  new Date(iso).toLocaleString('es-PE', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    hour: '2-digit',
    minute: '2-digit',
  })

export const fechaCorta = (iso) =>
  new Date(iso).toLocaleDateString('es-PE', { day: '2-digit', month: 'short', year: 'numeric' })

export const diaDelMes = (iso) => new Date(iso).toLocaleDateString('es-PE', { day: '2-digit' })

export const mesCorto = (iso) =>
  new Date(iso).toLocaleDateString('es-PE', { month: 'short' }).replace('.', '')

export const horaCorta = (iso) =>
  iso ? new Date(iso).toLocaleTimeString('es-PE', { hour: '2-digit', minute: '2-digit' }) : ''
