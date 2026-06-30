import { useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useCheckin } from '../hooks/useCheckin.js'
import { EscanerQR } from '../components/EscanerQR.jsx'
import { ResultadoCheckin } from '../components/ResultadoCheckin.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { IconoCamara, IconoAtras, IconoEnlace } from '../../../components/iconos/Iconos.jsx'
import { esTelefono } from '../../../lib/dispositivo.js'

// Modo puerta: escanea entradas una tras otra y muestra el resultado al instante.
export default function CheckinPage() {
  const { id } = useParams()
  const navegar = useNavigate()
  const { resultado, validar } = useCheckin(id)
  const [escaneando, setEscaneando] = useState(true)

  // El Modo puerta usa la cámara trasera para escanear en la puerta: solo tiene sentido
  // en un teléfono. En una laptop/escritorio mostramos cómo abrirlo en el celular.
  if (!esTelefono()) return <SoloTelefono id={id} navegar={navegar} />

  return (
    <div className="pila col-angosta">
      <div className="pila">
        <h1>Modo puerta</h1>
        <p>Escanea el código QR de cada entrada para validar el ingreso.</p>
      </div>

      <ResultadoCheckin resultado={resultado} />

      {escaneando ? (
        <EscanerQR onCodigo={validar} />
      ) : (
        <Boton ancho onClick={() => setEscaneando(true)}>
          <IconoCamara size={20} /> Activar cámara
        </Boton>
      )}

      <div className="pila">
        <Boton variante="secundario" onClick={() => setEscaneando((s) => !s)}>
          {escaneando ? 'Pausar cámara' : 'Reanudar cámara'}
        </Boton>
        <Boton variante="texto" onClick={() => navegar(`/panel/eventos/${id}`)}>
          <IconoAtras size={20} /> Volver al evento
        </Boton>
      </div>
    </div>
  )
}

// Pantalla que se muestra cuando se abre el Modo puerta fuera de un teléfono.
function SoloTelefono({ id, navegar }) {
  const [copiado, setCopiado] = useState(false)
  const enlace = window.location.href

  async function copiar() {
    try {
      await navigator.clipboard.writeText(enlace)
      setCopiado(true)
      setTimeout(() => setCopiado(false), 2000)
    } catch {
      setCopiado(false)
    }
  }

  return (
    <div className="pila col-angosta">
      <div className="pila">
        <h1>Modo puerta</h1>
        <p>
          Este modo usa la cámara para escanear las entradas en la puerta, así que
          <strong> solo funciona desde un teléfono</strong>. Ábrelo en tu celular para empezar.
        </p>
      </div>

      <Boton ancho onClick={copiar}>
        <IconoEnlace size={20} /> {copiado ? '¡Enlace copiado!' : 'Copiar enlace para el teléfono'}
      </Boton>

      <Boton variante="texto" onClick={() => navegar(`/panel/eventos/${id}`)}>
        <IconoAtras size={20} /> Volver al evento
      </Boton>
    </div>
  )
}
