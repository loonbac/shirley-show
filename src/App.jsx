import { lazy, Suspense } from 'react'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { ProveedorSesion, useSesion, LoginPage } from './features/auth/index.js'
import { AppShell } from './components/layout/AppShell.jsx'
import { RutaProtegida } from './routes/RutaProtegida.jsx'
import { Cargando } from './components/Estado.jsx'

// Cada feature se carga de forma diferida (code splitting): solo se descarga al visitarla.
const EventosPage = lazy(() => import('./features/eventos/pages/EventosPage.jsx'))
const FormularioEventoPage = lazy(() => import('./features/eventos/pages/FormularioEventoPage.jsx'))
const DetalleEventoPage = lazy(() => import('./features/eventos/pages/DetalleEventoPage.jsx'))
const CheckinPage = lazy(() => import('./features/checkin/pages/CheckinPage.jsx'))
const EntradaPublicaPage = lazy(() => import('./features/entradas/pages/EntradaPublicaPage.jsx'))

// Si ya hay sesión, el login redirige directo al panel.
function Inicio() {
  const { autenticado } = useSesion()
  return autenticado ? <Navigate to="/eventos" replace /> : <LoginPage />
}

export default function App() {
  return (
    <ProveedorSesion>
      <BrowserRouter>
        <Suspense fallback={<Cargando />}>
          <Routes>
            <Route path="/" element={<Inicio />} />
            {/* Vista pública de entrada (compartible por WhatsApp, sin login) */}
            <Route path="/entrada/:codigo" element={<EntradaPublicaPage />} />

            {/* Panel protegido */}
            <Route
              element={
                <RutaProtegida>
                  <AppShell />
                </RutaProtegida>
              }
            >
              <Route path="/eventos" element={<EventosPage />} />
              <Route path="/eventos/nuevo" element={<FormularioEventoPage />} />
              <Route path="/eventos/:id" element={<DetalleEventoPage />} />
              <Route path="/eventos/:id/editar" element={<FormularioEventoPage />} />
              <Route path="/eventos/:id/checkin" element={<CheckinPage />} />
            </Route>

            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Suspense>
      </BrowserRouter>
    </ProveedorSesion>
  )
}
