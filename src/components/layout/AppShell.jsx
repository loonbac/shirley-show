import { Outlet, Link, useNavigate, useLocation } from 'react-router-dom'
import { useSesion } from '../../features/auth/index.js'
import { Boton } from '../Boton.jsx'
import { IconoAtras } from '../iconos/Iconos.jsx'
import './AppShell.css'

// Armazón del panel: cabecera con marca + acción contextual y contenido de la ruta.
// En la página principal de eventos se muestra "Salir"; en las internas, "Atrás".
export function AppShell() {
  const navegar = useNavigate()
  const { pathname } = useLocation()
  const { cerrarSesion } = useSesion()
  const enRaiz = pathname === '/' || pathname === '/eventos'

  return (
    <div className="shell">
      <header className="shell__cabecera">
        <div className="shell__inner">
          <Link to="/eventos" className="shell__marca" aria-label="Shirley Show — inicio">
            <img className="shell__logo" src="/logo.png?v=2" alt="Shirley Show" />
          </Link>
          {enRaiz ? (
            <Boton variante="texto" onClick={cerrarSesion}>
              Salir
            </Boton>
          ) : (
            <Boton variante="texto" onClick={() => navegar(-1)}>
              <IconoAtras size={20} /> Atrás
            </Boton>
          )}
        </div>
      </header>
      <main className="contenedor">
        <Outlet />
      </main>
    </div>
  )
}
