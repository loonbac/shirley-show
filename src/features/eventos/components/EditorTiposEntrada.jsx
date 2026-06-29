import { Input } from '../../../components/Input.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { IconoMas, IconoEquis } from '../../../components/iconos/Iconos.jsx'
import './EditorTiposEntrada.css'

// Editor de la lista de tipos de entrada {nombre, precio}. Permite agregar y quitar filas.
export function EditorTiposEntrada({ tipos, onCambiar }) {
  const actualizar = (i, campo, valor) => {
    const copia = tipos.map((t, idx) => (idx === i ? { ...t, [campo]: valor } : t))
    onCambiar(copia)
  }
  const agregar = () => onCambiar([...tipos, { nombre: '', precio: '' }])
  const quitar = (i) => onCambiar(tipos.filter((_, idx) => idx !== i))

  return (
    <div className="pila">
      <span className="campo__etiqueta">Tipos de entrada y precio</span>
      {tipos.map((t, i) => (
        <div key={i} className="tipos__fila">
          <Input
            placeholder="Nombre (ej. VIP)"
            value={t.nombre}
            onChange={(e) => actualizar(i, 'nombre', e.target.value)}
          />
          <Input
            className="tipos__precio"
            type="number"
            min="0"
            placeholder="S/"
            value={t.precio}
            onChange={(e) => actualizar(i, 'precio', e.target.value)}
          />
          {tipos.length > 1 && (
            <button type="button" className="tipos__quitar" onClick={() => quitar(i)} aria-label="Quitar">
              <IconoEquis size={20} />
            </button>
          )}
        </div>
      ))}
      <Boton type="button" variante="secundario" onClick={agregar}>
        <IconoMas size={20} /> Agregar tipo de entrada
      </Boton>
    </div>
  )
}
