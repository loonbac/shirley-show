import { useNavigate } from 'react-router-dom'
import { Tarjeta } from '../../../components/Tarjeta.jsx'
import { Badge, tonoEvento } from '../../../components/Badge.jsx'
import { IconoUbicacion, IconoReloj } from '../../../components/iconos/Iconos.jsx'
import { diaDelMes, mesCorto, horaCorta } from '../../../lib/format.js'
import { urlImagen } from '../services/eventosApi.js'
import './TarjetaEvento.css'

// Tarjeta de un evento: afiche (si tiene), tile de fecha, datos y barra de aforo.
export function TarjetaEvento({ evento }) {
  const navegar = useNavigate()
  const finalizado = evento.estado === 'Finalizado'
  const pct = evento.aforo ? Math.min(Math.round((evento.vendidas / evento.aforo) * 100), 100) : null

  return (
    <Tarjeta
      className={`tev ${finalizado ? 'tev--fin' : ''} ${evento.imagen ? 'tev--img' : ''}`}
      onClick={() => navegar(`/eventos/${evento.id}`)}
    >
      {evento.imagen && (
        <div className="tev__poster">
          <img src={urlImagen(evento.imagen)} alt={evento.nombre} loading="lazy" />
        </div>
      )}
      <div className="tev__fila">
        <div className="tev__fecha">
          <span className="tev__dia">{diaDelMes(evento.fecha_hora)}</span>
          <span className="tev__mes">{mesCorto(evento.fecha_hora)}</span>
        </div>
        <div className="tev__cuerpo">
          <div className="tev__top">
            <h3 className="tev__nombre">{evento.nombre}</h3>
            <Badge tono={tonoEvento(evento.estado)}>{evento.estado}</Badge>
          </div>
          <p className="tev__meta"><IconoReloj size={18} /> {horaCorta(evento.fecha_hora)}</p>
          <p className="tev__meta"><IconoUbicacion size={18} /> {evento.lugar}</p>
        </div>
      </div>

      <div className="tev__aforo">
        {pct !== null && (
          <div className="tev__barra">
            <span className="tev__barra-fill" style={{ width: `${pct}%` }} />
          </div>
        )}
        <span className="tev__aforo-txt">
          <strong>{evento.vendidas}</strong>
          {evento.aforo != null ? ` / ${evento.aforo}` : ''} entradas vendidas
        </span>
      </div>
    </Tarjeta>
  )
}
