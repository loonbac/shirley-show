const FILL = 'url(#gradNota)'

// Una corchea (nota musical) dentro de la celda (x,y), en unidades de módulo (1x1).
// Cabeza redonda + plica vertical + banderín. Derecha y del mismo tamaño en todas,
// para que se lea claramente como una nota. La cabeza cubre el centro de la celda,
// así el lector la detecta como módulo oscuro (ver verificación de escaneo).
export function Nota({ x, y }) {
  return (
    <g fill={FILL}>
      {/* cabeza */}
      <circle cx={x + 0.42} cy={y + 0.66} r={0.27} />
      {/* plica */}
      <rect x={x + 0.62} y={y + 0.14} width={0.1} height={0.54} rx={0.04} />
      {/* banderín */}
      <path d={`M${x + 0.72} ${y + 0.14} Q${x + 0.98} ${y + 0.26} ${x + 0.84} ${y + 0.5} Q${x + 0.92} ${y + 0.3} ${x + 0.72} ${y + 0.28} Z`} />
    </g>
  )
}

// Patrón de detección (esquina): cuadrado exterior, hueco blanco y punto interior.
// Se dibuja sólido (no como notas) para que el escaneo sea siempre fiable.
export function Finder({ r0, c0 }) {
  return (
    <g>
      <rect x={c0 + 0.1} y={r0 + 0.1} width={6.8} height={6.8} rx={1.8} fill={FILL} />
      <rect x={c0 + 1.1} y={r0 + 1.1} width={4.8} height={4.8} rx={1.2} fill="#ffffff" />
      <rect x={c0 + 2} y={r0 + 2} width={3} height={3} rx={0.9} fill={FILL} />
    </g>
  )
}
