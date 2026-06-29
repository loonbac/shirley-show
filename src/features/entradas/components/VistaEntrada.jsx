import { VisorQR } from '../../../components/qr/VisorQR.jsx'
import { LineaDato } from '../../../components/LineaDato.jsx'
import { IconoCalendario, IconoUbicacion } from '../../../components/iconos/Iconos.jsx'
import { fechaLarga } from '../../../lib/format.js'
import './VistaEntrada.css'

// Vista limpia de una entrada, lista para compartir por WhatsApp (link o captura).
// `entrada` incluye datos del evento (nombre, fecha, lugar) cuando viene de la vista pública.
export function VistaEntrada({ entrada }) {
  const ev = entrada.evento
  return (
    <div className="ventrada">
      <VisorQR valor={entrada.codigo_qr} />
      <div className="ventrada__datos">
        <p className="ventrada__comprador">{entrada.nombre_comprador}</p>
        {ev && <p className="ventrada__evento">{ev.nombre}</p>}
        {ev && <LineaDato icono={IconoCalendario}>{fechaLarga(ev.fecha_hora)}</LineaDato>}
        {ev?.lugar && <LineaDato icono={IconoUbicacion}>{ev.lugar}</LineaDato>}
        <p className="ventrada__tipo">{entrada.tipo_entrada}</p>
        <p className="ventrada__codigo">Código: {entrada.codigo_qr.slice(0, 8).toUpperCase()}</p>
      </div>
    </div>
  )
}
