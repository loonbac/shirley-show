import { CINTA } from '../config.js'

// Cuántas veces se repiten las palabras dentro de UNA mitad. Así cada mitad es
// más ancha que la pantalla y el bucle nunca deja huecos en blanco.
const REPETICIONES = 4

// Una mitad de la cinta: la lista de palabras repetida varias veces.
function Mitad() {
  const palabras = Array.from({ length: REPETICIONES }, () => CINTA).flat()
  return (
    <div className="pres__cinta-grupo" aria-hidden="true">
      {palabras.map((t, i) => (
        <span key={`${t}-${i}`} style={{ display: 'contents' }}>
          <b>{t}</b>
          <i>♪</i>
        </span>
      ))}
    </div>
  )
}

// Cinta dorada en desplazamiento continuo. Se renderizan DOS mitades idénticas y
// la animación traslada exactamente -50% (una mitad), por lo que el ciclo es
// perfectamente continuo, sin saltos ni espacios vacíos.
export function CintaMarquee() {
  return (
    <div className="pres__cinta">
      <div className="pres__cinta-track">
        <Mitad />
        <Mitad />
      </div>
    </div>
  )
}
