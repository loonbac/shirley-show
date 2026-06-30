const FILL = 'url(#gradNota)'

// Un punto (círculo) centrado en la celda (x,y), en unidades de módulo (1x1).
// r=0.42 cubre el centro de la celda, así el lector lo detecta como módulo oscuro
// y se ve como "puntitos" (verificado que escanea).
export function Punto({ x, y }) {
  return <circle cx={x + 0.5} cy={y + 0.5} r={0.42} fill={FILL} />
}

// Patrón de detección (esquina): cuadrado exterior, hueco blanco y punto interior.
// Se dibuja sólido (no como puntos) para que el escaneo sea siempre fiable.
export function Finder({ r0, c0 }) {
  return (
    <g>
      <rect x={c0 + 0.1} y={r0 + 0.1} width={6.8} height={6.8} rx={1.8} fill={FILL} />
      <rect x={c0 + 1.1} y={r0 + 1.1} width={4.8} height={4.8} rx={1.2} fill="#ffffff" />
      <rect x={c0 + 2} y={r0 + 2} width={3} height={3} rx={0.9} fill={FILL} />
    </g>
  )
}
