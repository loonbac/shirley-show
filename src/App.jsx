import { lazy, Suspense } from 'react'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { ProveedorSesion, useSesion, LoginPage } from './features/auth/index.js'
import { AppShell } from './components/layout/AppShell.jsx'
import { RutaProtegida } from './routes/RutaProtegida.jsx'
import { Cargando } from './components/Estado.jsx'

// Cada feature se carga de forma diferida (code splitting): solo se descarga al visitarla.
const PresentacionPage = lazy(() => import('./features/presentacion/pages/PresentacionPage.jsx'))
const EventosPage = lazy(() => import('./features/eventos/pages/EventosPage.jsx'))
const FormularioEventoPage = lazy(() => import('./features/eventos/pages/FormularioEventoPage.jsx'))
const DetalleEventoPage = lazy(() => import('./features/eventos/pages/DetalleEventoPage.jsx'))
const CheckinPage = lazy(() => import('./features/checkin/pages/CheckinPage.jsx'))
const EntradaPublicaPage = lazy(() => import('./features/entradas/pages/EntradaPublicaPage.jsx'))

// Entrada al panel (/panel): si ya hay sesión, salta directo a la lista de eventos.
function Panel() {
  const { autenticado } = useSesion()
  return autenticado ? <Navigate to="/panel/eventos" replace /> : <LoginPage />
}

export default function App() {
  return (
    <ProveedorSesion>
      <BrowserRouter>
        <Suspense fallback={<Cargando />}>
          <Routes>
            {/* Portada pública: presentación de la artista (sin login) */}
            <Route path="/" element={<PresentacionPage />} />
            {/* Vista pública de entrada (compartible por WhatsApp, sin login) */}
            <Route path="/entrada/:codigo" element={<EntradaPublicaPage />} />

            {/* Login del panel */}
            <Route path="/panel" element={<Panel />} />

            {/* Panel protegido (todo bajo /panel) */}
            <Route
              element={
                <RutaProtegida>
                  <AppShell />
                </RutaProtegida>
              }
            >
              <Route path="/panel/eventos" element={<EventosPage />} />
              <Route path="/panel/eventos/nuevo" element={<FormularioEventoPage />} />
              <Route path="/panel/eventos/:id" element={<DetalleEventoPage />} />
              <Route path="/panel/eventos/:id/editar" element={<FormularioEventoPage />} />
              <Route path="/panel/eventos/:id/checkin" element={<CheckinPage />} />
            </Route>

            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Suspense>
      </BrowserRouter>
    </ProveedorSesion>
  )
}
