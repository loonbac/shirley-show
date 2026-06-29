// Opciones fijas del dominio de entradas (deben coincidir con el backend).
export const MEDIOS_PAGO = ['Yape', 'Efectivo', 'Otra billetera']
export const ESTADOS_PAGO = ['Pagado', 'Pendiente']

export const opciones = (lista) => lista.map((v) => ({ valor: v, etiqueta: v }))
