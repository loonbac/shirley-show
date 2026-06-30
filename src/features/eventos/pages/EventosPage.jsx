import { useNavigate } from 'react-router-dom'
import { useEventos } from '../hooks/useEventos.js'
import { ListaEventos } from '../components/ListaEventos.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { IconoMas } from '../../../components/iconos/Iconos.jsx'
import { Cargando, MensajeError } from '../../../components/Estado.jsx'
import './EventosPage.css'

// Pantalla principal: lista de eventos agrupados, con botón para crear uno nuevo.
export default function EventosPage() {
  const navegar = useNavigate()
  const { datos: eventos, cargando, error } = useEventos()

  return (
    <div className="pila">
      <header className="barra-pagina eventos-head">
        <div className="eventos-head__txt">
          <h1>Eventos</h1>
          <p className="eventos-head__sub">Tus shows y la venta de entradas, en un solo lugar.</p>
        </div>
        <Boton ancho onClick={() => navegar('/panel/eventos/nuevo')}>
          <IconoMas size={22} /> Crear evento
        </Boton>
      </header>
      {cargando && <Cargando />}
      {error && <MensajeError>{error}</MensajeError>}
      {eventos && <ListaEventos eventos={eventos} />}
    </div>
  )
}
