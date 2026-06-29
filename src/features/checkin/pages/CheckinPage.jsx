import { useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useCheckin } from '../hooks/useCheckin.js'
import { EscanerQR } from '../components/EscanerQR.jsx'
import { ResultadoCheckin } from '../components/ResultadoCheckin.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { IconoCamara, IconoAtras } from '../../../components/iconos/Iconos.jsx'

// Modo puerta: escanea entradas una tras otra y muestra el resultado al instante.
export default function CheckinPage() {
  const { id } = useParams()
  const navegar = useNavigate()
  const { resultado, validar } = useCheckin(id)
  const [escaneando, setEscaneando] = useState(true)

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
        <Boton variante="texto" onClick={() => navegar(`/eventos/${id}`)}>
          <IconoAtras size={20} /> Volver al evento
        </Boton>
      </div>
    </div>
  )
}
