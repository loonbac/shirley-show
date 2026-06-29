import { IconoValida, IconoAlerta, IconoInvalida } from '../../../components/iconos/Iconos.jsx'
import { horaCorta } from '../../../lib/format.js'
import './ResultadoCheckin.css'

// Muestra el resultado del último escaneo, grande y con color, para verlo de un vistazo.
const CONFIG = {
  valida: { Icono: IconoValida, titulo: '¡Entrada válida!', clase: 'ok' },
  usada: { Icono: IconoAlerta, titulo: 'Ya fue usada', clase: 'aviso' },
  invalida: { Icono: IconoInvalida, titulo: 'Entrada inválida', clase: 'mal' },
}

export function ResultadoCheckin({ resultado }) {
  if (!resultado) {
    return <div className="rcheck rcheck--espera">Apunta la cámara al QR de la entrada</div>
  }
  const c = CONFIG[resultado.resultado]
  const e = resultado.entrada
  return (
    <div className={`rcheck rcheck--${c.clase}`}>
      <c.Icono size={56} className="rcheck__icono" />
      <strong className="rcheck__titulo">{c.titulo}</strong>
      {e && <span className="rcheck__nombre">{e.nombre_comprador} · {e.tipo_entrada}</span>}
      {resultado.resultado === 'usada' && e?.hora_ingreso && (
        <span className="rcheck__hora">Ingresó a las {horaCorta(e.hora_ingreso)}</span>
      )}
      {resultado.resultado === 'valida' && <span className="rcheck__hora">Adelante, ¡que disfrute!</span>}
    </div>
  )
}
