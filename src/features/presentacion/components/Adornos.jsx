// Adornos decorativos reutilizables de la landing: nota musical y onda de sonido.

// Nota musical de línea (motivo de marca). `size` y `stroke` configurables.
export function IconoNota({ size = 24, stroke = '#9a7110', width = 1.6, style, className }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" style={style} className={className}>
      <path d="M9 17V5l10-2v12" stroke={stroke} strokeWidth={width} strokeLinecap="round" strokeLinejoin="round" />
      <circle cx="6.5" cy="17" r="2.6" stroke={stroke} strokeWidth={width} />
      <circle cx="16.5" cy="15" r="2.6" stroke={stroke} strokeWidth={width} />
    </svg>
  )
}

// Barras tipo ecualizador. `alturas` define cada barra; `ancho`/`opacidades` ajustables.
export function OndaSonido({ alturas = [9, 16, 22, 14, 20, 11], ancho = 3, opacidades = [0.45, 0.55, 0.7, 0.55, 0.65, 0.45] }) {
  return (
    <div className="pres__onda">
      {alturas.map((h, i) => (
        <span key={i} style={{ height: `${h}px`, width: `${ancho}px`, opacity: opacidades[i % opacidades.length] }} />
      ))}
    </div>
  )
}
