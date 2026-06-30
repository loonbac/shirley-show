import { useNavigate } from 'react-router-dom'
import { Badge, tonoEvento } from '../../../components/Badge.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { LineaDato } from '../../../components/LineaDato.jsx'
import {
  IconoCalendario, IconoUbicacion, IconoCamara, IconoLapiz, IconoBandera, IconoReloj, IconoBasura,
} from '../../../components/iconos/Iconos.jsx'
import { fechaLarga } from '../../../lib/format.js'
import { urlImagen } from '../services/eventosApi.js'
import './CabeceraEvento.css'

// Cabecera del detalle: datos del evento + acciones (puerta, editar, finalizar/reabrir, eliminar).
export function CabeceraEvento({ evento, accionando, onFinalizar, onReabrir, onEliminar }) {
  const navegar = useNavigate()
  return (
    <header className="cab">
      {evento.imagen && (
        <img className="cab__poster" src={urlImagen(evento.imagen)} alt={`Afiche de ${evento.nombre}`} />
      )}
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
        {evento.finalizado ? (
          <Boton variante="secundario" onClick={onReabrir} disabled={accionando}>
            <IconoReloj size={20} /> Reabrir
          </Boton>
        ) : (
          <Boton variante="secundario" onClick={onFinalizar} disabled={accionando}>
            <IconoBandera size={20} /> Finalizar
          </Boton>
        )}
        <Boton variante="peligro" onClick={onEliminar} disabled={accionando}>
          <IconoBasura size={20} /> Eliminar
        </Boton>
      </div>
    </header>
  )
}
