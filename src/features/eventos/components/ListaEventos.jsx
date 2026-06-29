import { TarjetaEvento } from './TarjetaEvento.jsx'
import { Vacio } from '../../../components/Estado.jsx'
import { IconoCalendario, IconoReloj } from '../../../components/iconos/Iconos.jsx'
import './ListaEventos.css'

// Agrupa los eventos en "Próximos" y "Finalizados", cada grupo con su icono y contador.
export function ListaEventos({ eventos }) {
  const proximos = eventos.filter((e) => e.estado === 'Próximo')
  const finalizados = eventos.filter((e) => e.estado === 'Finalizado')

  if (eventos.length === 0) {
    return <Vacio>Aún no hay eventos. Crea el primero con el botón “Crear evento”.</Vacio>
  }

  return (
    <div className="pila">
      <Grupo titulo="Próximos" icono={IconoCalendario} items={proximos} />
      <Grupo titulo="Finalizados" icono={IconoReloj} items={finalizados} />
    </div>
  )
}

function Grupo({ titulo, icono: Icono, items }) {
  if (items.length === 0) return null
  return (
    <section>
      <div className="grupo__head">
        <span className="grupo__icono"><Icono size={22} /></span>
        <h2>{titulo}</h2>
        <span className="grupo__chip">{items.length}</span>
      </div>
      <div className="eventos-grid">
        {items.map((e) => (
          <TarjetaEvento key={e.id} evento={e} />
        ))}
      </div>
    </section>
  )
}
