import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { EditorTiposEntrada } from './EditorTiposEntrada.jsx'
import { urlImagen } from '../services/eventosApi.js'
import './FormularioEvento.css'

// Separa un ISO en sus partes locales de fecha (YYYY-MM-DD) y hora (HH:MM), para
// usar dos inputs distintos (calendario + reloj), más claros que datetime-local.
const aPartes = (iso) => {
  if (!iso) return { fecha: '', hora: '' }
  const d = new Date(iso)
  const local = new Date(d - d.getTimezoneOffset() * 60000).toISOString()
  return { fecha: local.slice(0, 10), hora: local.slice(11, 16) }
}

// Formulario reutilizable para crear y editar un evento.
export function FormularioEvento({ inicial, guardando, onGuardar }) {
  const ini = aPartes(inicial?.fecha_hora)
  const [nombre, setNombre] = useState(inicial?.nombre || '')
  const [fecha, setFecha] = useState(ini.fecha)
  const [hora, setHora] = useState(ini.hora)
  const [lugar, setLugar] = useState(inicial?.lugar || '')
  const [tipos, setTipos] = useState(inicial?.tipos_de_entrada || [{ nombre: 'General', precio: '' }])
  const [imagenFile, setImagenFile] = useState(null)
  const [preview, setPreview] = useState(urlImagen(inicial?.imagen))
  const [error, setError] = useState('')

  function elegirImagen(e) {
    const file = e.target.files?.[0]
    if (!file) return
    if (!file.type.startsWith('image/')) return setError('El afiche debe ser una imagen (JPG, PNG o WebP)')
    setError('')
    setImagenFile(file)
    setPreview(URL.createObjectURL(file))
  }

  function quitarImagen() {
    setImagenFile(null)
    setPreview(null)
  }

  function enviar(e) {
    e.preventDefault()
    setError('')
    if (!fecha) return setError('Elige la fecha del evento')
    if (!hora) return setError('Elige la hora del evento')
    const fechaHora = new Date(`${fecha}T${hora}`)
    if (isNaN(fechaHora.getTime())) return setError('La fecha u hora no es válida')
    onGuardar(
      {
        nombre,
        fecha_hora: fechaHora.toISOString(),
        lugar,
        tipos_de_entrada: tipos.map((t) => ({ nombre: t.nombre.trim(), precio: Number(t.precio) })),
      },
      imagenFile,
    ).catch((err) => setError(err.message))
  }

  return (
    <form className="pila" onSubmit={enviar}>
      <Campo etiqueta="Nombre del evento" htmlFor="nombre">
        <Input id="nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} autoFocus />
      </Campo>

      <div className="campo">
        <span className="campo__etiqueta">Afiche del evento (opcional)</span>
        <div className="afiche">
          <label className="afiche__zona">
            {preview ? (
              <img className="afiche__preview" src={preview} alt="Afiche del evento" />
            ) : (
              <span className="afiche__placeholder">Toca para elegir una imagen vertical</span>
            )}
            <input type="file" accept="image/jpeg,image/png,image/webp" onChange={elegirImagen} hidden />
          </label>
          {preview && (
            <Boton type="button" variante="secundario" onClick={quitarImagen}>
              Quitar afiche
            </Boton>
          )}
        </div>
        <span className="campo__ayuda">Imagen vertical de la publicidad. JPG, PNG o WebP, hasta 10 MB.</span>
      </div>

      <div className="campo-fila">
        <Campo etiqueta="Fecha" htmlFor="fecha">
          <Input id="fecha" type="date" value={fecha} onChange={(e) => setFecha(e.target.value)} />
        </Campo>
        <Campo etiqueta="Hora" htmlFor="hora">
          <Input id="hora" type="time" value={hora} onChange={(e) => setHora(e.target.value)} />
        </Campo>
      </div>
      <Campo etiqueta="Lugar" htmlFor="lugar">
        <Input id="lugar" value={lugar} onChange={(e) => setLugar(e.target.value)} />
      </Campo>
      <EditorTiposEntrada tipos={tipos} onCambiar={setTipos} />
      {error && <MensajeError>{error}</MensajeError>}
      <Boton type="submit" ancho disabled={guardando}>
        {guardando ? 'Guardando…' : 'Guardar evento'}
      </Boton>
    </form>
  )
}
