import { Tarjeta } from '../../../components/Tarjeta.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { IconoLapiz, IconoBasura } from '../../../components/iconos/Iconos.jsx'
import { soles } from '../../../lib/format.js'

// Un gasto en la lista, con su monto y acciones (editar / quitar).
// Reutiliza las clases de tarjeta de las entradas (fila__*, entradas-grid).
export function FilaGasto({ gasto, onEditar, onEliminar, ocupado }) {
  return (
    <Tarjeta>
      <div className="fila__top">
        <strong className="fila__nombre">{gasto.categoria}</strong>
        <span className="gasto__monto">{soles(gasto.monto)}</span>
      </div>
      {gasto.descripcion && <p className="gasto__desc">{gasto.descripcion}</p>}
      <div className="fila__acciones">
        <Boton variante="secundario" onClick={() => onEditar(gasto)} disabled={ocupado}>
          <IconoLapiz size={18} /> Editar
        </Boton>
        <Boton variante="texto" onClick={() => onEliminar(gasto)} disabled={ocupado}>
          <IconoBasura size={18} /> Quitar
        </Boton>
      </div>
    </Tarjeta>
  )
}
