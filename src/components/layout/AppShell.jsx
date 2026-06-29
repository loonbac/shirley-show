import { Outlet, Link } from 'react-router-dom'
import { useSesion } from '../../features/auth/index.js'
import { Boton } from '../Boton.jsx'
import { IconoNota } from '../iconos/Iconos.jsx'
import './AppShell.css'

// Armazón del panel: cabecera con marca y cierre de sesión + contenido de la ruta.
export function AppShell() {
  const { cerrarSesion } = useSesion()
  return (
    <div className="shell">
      <header className="shell__cabecera">
        <div className="shell__inner">
          <Link to="/eventos" className="shell__marca">
            <IconoNota size={24} className="shell__nota" /> Shirley Show
          </Link>
          <Boton variante="texto" onClick={cerrarSesion}>
            Salir
          </Boton>
        </div>
      </header>
      <main className="contenedor">
        <Outlet />
      </main>
    </div>
  )
}
