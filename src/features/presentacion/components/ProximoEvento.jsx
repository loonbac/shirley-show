import { useProximoEvento } from '../hooks/useProximoEvento.js'
import { urlImagenEvento } from '../services/proximoEventoApi.js'
import { ImagenMedio } from './ImagenMedio.jsx'
import { fechaLarga } from '../../../lib/format.js'

const mayúscInicial = (s) => s.charAt(0).toUpperCase() + s.slice(1)

// Destaca el evento más cercano cargado en el panel (si hay alguno), con su afiche.
// No se muestra nada si aún no hay un próximo evento programado.
export function ProximoEvento() {
  const { evento, cargando } = useProximoEvento()

  if (cargando || !evento) return null

  return (
    <section className="pres__sec pres__sec--crema">
      <div className="pres__wrap pres__proximo">
        <div className="pres__proximo-foto">
          <ImagenMedio
            src={urlImagenEvento(evento.imagen)}
            titulo={evento.nombre}
            alt={`Afiche de ${evento.nombre}`}
          />
        </div>
        <div className="pres__proximo-texto">
          <span className="pres__chip pres__chip--blanco">Próximo evento</span>
          <h2 className="pres__h2">{evento.nombre}</h2>
          <p className="pres__lead pres__proximo-fecha">{mayúscInicial(fechaLarga(evento.fecha_hora))}</p>
          <p className="pres__lead">{evento.lugar}</p>
        </div>
      </div>
    </section>
  )
}
