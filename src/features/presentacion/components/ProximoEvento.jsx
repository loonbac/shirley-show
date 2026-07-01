import { useState } from 'react'
import { useProximoEvento } from '../hooks/useProximoEvento.js'
import { urlImagenEvento } from '../services/proximoEventoApi.js'
import { ImagenMedio } from './ImagenMedio.jsx'
import { LightboxGaleria } from './LightboxGaleria.jsx'
import { diaDelMes, mesCorto, horaCorta } from '../../../lib/format.js'

// Tarjeta del evento más cercano cargado en el panel, pensada para ir en el hero
// (primera pantalla). No se muestra nada si todavía no hay un próximo evento.
export function ProximoEvento() {
  const { evento, cargando } = useProximoEvento()
  const [abierto, setAbierto] = useState(false)

  if (cargando || !evento) return null

  const urlImagen = urlImagenEvento(evento.imagen)

  return (
    <>
      <div className="pres__hero-divisor" aria-hidden="true" />
      <div className="pres__hero-evento">
        <span className="pres__eyebrow">Próximo evento</span>
        {urlImagen ? (
          <button
            type="button"
            className="pres__hero-evento-img"
            onClick={() => setAbierto(true)}
            aria-label={`Ver afiche de ${evento.nombre} en pantalla completa`}
          >
            <ImagenMedio src={urlImagen} titulo={evento.nombre} alt={`Afiche de ${evento.nombre}`} />
          </button>
        ) : (
          <div className="pres__hero-evento-img">
            <ImagenMedio titulo={evento.nombre} alt={`Afiche de ${evento.nombre}`} />
          </div>
        )}
        <h3 className="pres__hero-evento-nombre">{evento.nombre}</h3>
        <p className="pres__hero-evento-fecha">
          <span className="pres__hero-evento-dia">{diaDelMes(evento.fecha_hora)}</span>
          <span className="pres__hero-evento-mes">{mesCorto(evento.fecha_hora)}</span>
          <span>· {horaCorta(evento.fecha_hora)}</span>
        </p>
        <p className="pres__hero-evento-lugar">{evento.lugar}</p>
      </div>

      <LightboxGaleria
        item={abierto ? { src: urlImagen, titulo: evento.nombre, grande: true } : null}
        onCerrar={() => setAbierto(false)}
      />
    </>
  )
}
