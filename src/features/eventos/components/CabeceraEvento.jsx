import { useNavigate } from 'react-router-dom'
import { Badge, tonoEvento } from '../../../components/Badge.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { LineaDato } from '../../../components/LineaDato.jsx'
import { IconoCalendario, IconoUbicacion, IconoCamara, IconoLapiz } from '../../../components/iconos/Iconos.jsx'
import { fechaLarga } from '../../../lib/format.js'
import './CabeceraEvento.css'

// Cabecera del detalle: datos del evento + accesos a editar y al modo puerta.
export function CabeceraEvento({ evento }) {
  const navegar = useNavigate()
  return (
    <header className="cab">
      <div className="cab__titulo">
        <h1>{evento.nombre}</h1>
        <Badge tono={tonoEvento(evento.estado)}>{evento.estado}</Badge>
      </div>
      <LineaDato icono={IconoCalendario}>{fechaLarga(evento.fecha_hora)}</LineaDato>
      <LineaDato icono={IconoUbicacion}>{evento.lugar}</LineaDato>
      <div className="cab__acciones">
        <Boton variante="primario" onClick={() => navegar(`/eventos/${evento.id}/checkin`)}>
          <IconoCamara size={20} /> Modo puerta
        </Boton>
        <Boton variante="secundario" onClick={() => navegar(`/eventos/${evento.id}/editar`)}>
          <IconoLapiz size={20} /> Editar
        </Boton>
      </div>
    </header>
  )
}
