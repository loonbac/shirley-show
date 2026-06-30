// Detección de dispositivo para usos de UX (no es una barrera de seguridad).

// esTelefono indica si el navegador corre en un teléfono. Se usa para el Modo puerta,
// que necesita la cámara trasera y solo tiene sentido en el celular del personal.
// Heurística por User-Agent: Android con "Mobile" (los tablets Android no lo llevan),
// iPhone/iPod y otros móviles. No bloquea por seguridad (eso lo hace el login + el token
// del servidor), solo evita abrirlo en una laptop donde no sirve.
export function esTelefono() {
  if (typeof navigator === 'undefined') return false
  const ua = navigator.userAgent || ''
  return /iPhone|iPod|Android.*Mobile|Windows Phone|BlackBerry|BB10|Opera Mini|IEMobile/i.test(ua)
}
