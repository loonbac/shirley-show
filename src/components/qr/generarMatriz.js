import qrcode from 'qrcode-generator'

// Genera la matriz del QR (nivel H = máxima corrección) y devuelve helpers.
export function generarMatriz(valor) {
  const qr = qrcode(0, 'H') // 0 = autodetecta el tamaño según los datos
  qr.addData(valor)
  qr.make()
  const size = qr.getModuleCount()
  return { size, dark: (r, c) => qr.isDark(r, c) }
}

// ¿(r,c) cae en uno de los tres patrones de detección (esquinas 7x7)?
export function esEsquinaDeteccion(r, c, size) {
  const enBloque = (r0, c0) => r >= r0 && r < r0 + 7 && c >= c0 && c < c0 + 7
  return enBloque(0, 0) || enBloque(0, size - 7) || enBloque(size - 7, 0)
}
