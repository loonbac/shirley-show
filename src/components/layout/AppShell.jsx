import { Outlet, Link, useNavigate } from 'react-router-dom'
import { Boton } from '../Boton.jsx'
import { IconoNota, IconoAtras } from '../iconos/Iconos.jsx'
import './AppShell.css'

// Armazón del panel: cabecera con marca y botón "Atrás" + contenido de la ruta.
export function AppShell() {
  const navegar = useNavigate()
  return (
    <div className="shell">
      <header className="shell__cabecera">
        <div className="shell__inner">
          <Link to="/eventos" className="shell__marca">
            <IconoNota size={24} className="shell__nota" /> Shirley Show
          </Link>
          <Boton variante="texto" onClick={() => navegar(-1)}>
            <IconoAtras size={20} /> Atrás
          </Boton>
        </div>
      </header>
      <main className="contenedor">
        <Outlet />
      </main>
    </div>
  )
}
