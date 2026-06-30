import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { EditorTiposEntrada } from './EditorTiposEntrada.jsx'
import { urlImagen } from '../services/eventosApi.js'
import './FormularioEvento.css'

// Convierte ISO <-> valor del input datetime-local (hora local del dispositivo).
const aLocal = (iso) => {
  if (!iso) return ''
  const d = new Date(iso)
  const off = d.getTimezoneOffset() * 60000
  return new Date(d - off).toISOString().slice(0, 16)
}

// Formulario reutilizable para crear y editar un evento.
export function FormularioEvento({ inicial, guardando, onGuardar }) {
  const [nombre, setNombre] = useState(inicial?.nombre || '')
  const [fecha, setFecha] = useState(aLocal(inicial?.fecha_hora))
  const [lugar, setLugar] = useState(inicial?.lugar || '')
  const [aforo, setAforo] = useState(inicial?.aforo ?? '')
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
    if (!fecha) return setError('Indica la fecha y hora del evento')
    onGuardar(
      {
        nombre,
        fecha_hora: new Date(fecha).toISOString(),
        lugar,
        aforo: aforo === '' ? null : Number(aforo),
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

      <Campo etiqueta="Fecha y hora" htmlFor="fecha">
        <Input id="fecha" type="datetime-local" value={fecha} onChange={(e) => setFecha(e.target.value)} />
      </Campo>
      <Campo etiqueta="Lugar" htmlFor="lugar">
        <Input id="lugar" value={lugar} onChange={(e) => setLugar(e.target.value)} />
      </Campo>
      <Campo etiqueta="Aforo (opcional)" htmlFor="aforo" ayuda="Capacidad máxima. Déjalo vacío si no hay límite.">
        <Input id="aforo" type="number" min="0" value={aforo} onChange={(e) => setAforo(e.target.value)} />
      </Campo>
      <EditorTiposEntrada tipos={tipos} onCambiar={setTipos} />
      {error && <MensajeError>{error}</MensajeError>}
      <Boton type="submit" ancho disabled={guardando}>
        {guardando ? 'Guardando…' : 'Guardar evento'}
      </Boton>
    </form>
  )
}
