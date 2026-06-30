import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { CATEGORIAS_GASTO } from '../constantes.js'

// Formulario para agregar (o editar) un gasto del evento.
export function FormularioGasto({ inicial, guardando, onGuardar, onCancelar }) {
  const [categoria, setCategoria] = useState(inicial?.categoria || '')
  const [descripcion, setDescripcion] = useState(inicial?.descripcion || '')
  const [monto, setMonto] = useState(inicial?.monto ?? '')
  const [error, setError] = useState('')

  function enviar(e) {
    e.preventDefault()
    setError('')
    if (!categoria) return setError('Elige en qué se gastó')
    const valor = Number(monto)
    if (!valor || valor <= 0) return setError('Escribe un monto mayor que cero')
    onGuardar({
      categoria,
      descripcion: descripcion.trim() || null,
      monto: valor,
    }).catch((err) => setError(err.message))
  }

  return (
    <form className="pila" onSubmit={enviar}>
      <Campo etiqueta="¿En qué se gastó?">
        <div className="gasto-categorias">
          {CATEGORIAS_GASTO.map((c) => (
            <Boton
              key={c}
              type="button"
              variante={categoria === c ? 'primario' : 'secundario'}
              onClick={() => setCategoria(c)}
            >
              {c}
            </Boton>
          ))}
        </div>
      </Campo>
      <Campo etiqueta="Monto (S/)" htmlFor="gasto-monto">
        <Input
          id="gasto-monto"
          type="number"
          min="0"
          step="0.01"
          inputMode="decimal"
          value={monto}
          onChange={(e) => setMonto(e.target.value)}
          autoFocus
        />
      </Campo>
      <Campo etiqueta="Detalle (opcional)" htmlFor="gasto-desc" ayuda="Ej: nombre del artista, qué se compró, etc.">
        <Input id="gasto-desc" value={descripcion} onChange={(e) => setDescripcion(e.target.value)} />
      </Campo>
      {error && <MensajeError>{error}</MensajeError>}
      <Boton type="submit" ancho disabled={guardando}>
        {guardando ? 'Guardando…' : 'Guardar gasto'}
      </Boton>
      {onCancelar && (
        <Boton variante="secundario" ancho type="button" onClick={onCancelar} disabled={guardando}>
          Cancelar
        </Boton>
      )}
    </form>
  )
}
