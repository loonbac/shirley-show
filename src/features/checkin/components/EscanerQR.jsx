import { useEffect, useRef, useState } from 'react'
import { Html5Qrcode } from 'html5-qrcode'
import { MensajeError } from '../../../components/Estado.jsx'
import './EscanerQR.css'

const ID = 'lector-qr'

// Cámara que escanea QR de forma continua. Llama a `onCodigo` con cada lectura.
export function EscanerQR({ onCodigo }) {
  const [error, setError] = useState('')
  const onCodigoRef = useRef(onCodigo)
  onCodigoRef.current = onCodigo

  useEffect(() => {
    const lector = new Html5Qrcode(ID)
    let detenido = false

    lector
      .start(
        { facingMode: 'environment' },
        { fps: 10, qrbox: { width: 250, height: 250 } },
        (texto) => onCodigoRef.current(texto),
        () => {}, // errores por fotograma: se ignoran (es normal mientras enfoca)
      )
      .catch(() => setError('No se pudo abrir la cámara. Revisa los permisos del navegador.'))

    return () => {
      detenido = true
      // Detiene la cámara solo si llegó a iniciarse, para evitar errores de carrera.
      if (lector.getState && lector.getState() === 2) {
        lector.stop().then(() => lector.clear()).catch(() => {})
      } else {
        setTimeout(() => {
          if (detenido) lector.stop().then(() => lector.clear()).catch(() => {})
        }, 300)
      }
    }
  }, [])

  if (error) return <MensajeError>{error}</MensajeError>
  return <div id={ID} className="lector" />
}
