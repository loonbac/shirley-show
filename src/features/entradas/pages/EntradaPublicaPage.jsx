import { useParams } from 'react-router-dom'
import { useCallback } from 'react'
import { usePeticion } from '../../../hooks/usePeticion.js'
import { entradasApi } from '../services/entradasApi.js'
import { VistaEntrada } from '../components/VistaEntrada.jsx'
import { Cargando, MensajeError } from '../../../components/Estado.jsx'
import './EntradaPublicaPage.css'

// Página pública (sin login): muestra una entrada por su código, lista para compartir.
export default function EntradaPublicaPage() {
  const { codigo } = useParams()
  const { datos: entrada, cargando, error } = usePeticion(
    useCallback(() => entradasApi.porCodigo(codigo), [codigo]),
  )

  return (
    <div className="entrada-publica">
      {cargando && <Cargando />}
      {error && <MensajeError>Entrada no encontrada o link inválido.</MensajeError>}
      {entrada && <VistaEntrada entrada={entrada} />}
    </div>
  )
}
