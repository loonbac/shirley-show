// Librería de iconos SVG reutilizables. Toman el color del texto (currentColor).
// `size` controla el tamaño en píxeles. Reemplazan a todos los emojis de la app.

function Linea({ size = 24, children, ...p }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden {...p}>
      {children}
    </svg>
  )
}

function Solido({ size = 24, children, ...p }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden {...p}>
      {children}
    </svg>
  )
}

export const IconoNota = (p) => (
  <Solido {...p}>
    <rect x="10" y="4" width="2" height="12.5" rx="1" />
    <path d="M12 4c3 .5 6 2.1 5 6-.3-2.4-2.6-3.4-5-3.7z" />
    <ellipse cx="8" cy="17.4" rx="3.2" ry="2.6" />
  </Solido>
)

export const IconoCalendario = (p) => (
  <Linea {...p}>
    <rect x="3" y="4.5" width="18" height="16.5" rx="2" />
    <path d="M3 9.5h18M8 2.5v4M16 2.5v4" />
  </Linea>
)

export const IconoUbicacion = (p) => (
  <Linea {...p}>
    <path d="M12 21s7-6.3 7-11a7 7 0 1 0-14 0c0 4.7 7 11 7 11z" />
    <circle cx="12" cy="10" r="2.5" />
  </Linea>
)

export const IconoTicket = (p) => (
  <Linea {...p}>
    <path d="M3 8a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2 2 2 0 0 0 0 4 2 2 0 0 1-2 2H5a2 2 0 0 1-2-2 2 2 0 0 0 0-4z" />
    <path d="M9.5 6v12" strokeDasharray="2 2" />
  </Linea>
)

export const IconoCamara = (p) => (
  <Linea {...p}>
    <path d="M3 9a2 2 0 0 1 2-2h1.5L8 5h8l1.5 2H19a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    <circle cx="12" cy="13" r="3.5" />
  </Linea>
)

export const IconoLapiz = (p) => (
  <Linea {...p}>
    <path d="M4 20h4L19 9a2 2 0 0 0-3-3L5 17z" />
    <path d="M14 7l3 3" />
  </Linea>
)

export const IconoMas = (p) => (
  <Linea {...p}>
    <path d="M12 5v14M5 12h14" />
  </Linea>
)

export const IconoCheck = (p) => (
  <Linea {...p}>
    <path d="M5 13l4 4L19 7" />
  </Linea>
)

export const IconoEquis = (p) => (
  <Linea {...p}>
    <path d="M6 6l12 12M18 6L6 18" />
  </Linea>
)

export const IconoValida = (p) => (
  <Solido {...p}>
    <circle cx="12" cy="12" r="10" />
    <path d="M7 12.4l3.3 3.3L17 9" fill="none" stroke="#fff" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />
  </Solido>
)

export const IconoAlerta = (p) => (
  <Solido {...p}>
    <path d="M12 3l10 17H2z" />
    <path d="M12 9.5v4.5" stroke="#fff" strokeWidth="2.2" strokeLinecap="round" />
    <circle cx="12" cy="17" r="1.2" fill="#fff" />
  </Solido>
)

export const IconoInvalida = (p) => (
  <Solido {...p}>
    <circle cx="12" cy="12" r="10" />
    <path d="M8.5 8.5l7 7M15.5 8.5l-7 7" fill="none" stroke="#fff" strokeWidth="2.4" strokeLinecap="round" />
  </Solido>
)

export const IconoBandera = (p) => (
  <Linea {...p}>
    <path d="M5 21V4M5 4h11l-2 4 2 4H5" />
  </Linea>
)

export const IconoBasura = (p) => (
  <Linea {...p}>
    <path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13" />
  </Linea>
)

export const IconoReloj = (p) => (
  <Linea {...p}>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7.5v5l3 2" />
  </Linea>
)

export const IconoAtras = (p) => (
  <Linea {...p}>
    <path d="M15 5l-7 7 7 7" />
  </Linea>
)

export const IconoEnlace = (p) => (
  <Linea {...p}>
    <path d="M10 13a5 5 0 0 0 7 0l2-2a5 5 0 0 0-7-7l-1 1" />
    <path d="M14 11a5 5 0 0 0-7 0l-2 2a5 5 0 0 0 7 7l1-1" />
  </Linea>
)

export const IconoWhatsApp = (p) => (
  <Solido {...p}>
    <path d="M.057 24l1.687-6.163a11.867 11.867 0 0 1-1.587-5.946C.16 5.335 5.495 0 12.05 0a11.82 11.82 0 0 1 8.413 3.488 11.82 11.82 0 0 1 3.48 8.414c-.003 6.557-5.338 11.892-11.893 11.892a11.9 11.9 0 0 1-5.688-1.448L.057 24zm6.597-3.807c1.676.995 3.276 1.591 5.392 1.592 5.448 0 9.886-4.434 9.889-9.885.002-5.462-4.415-9.89-9.881-9.892-5.452 0-9.887 4.434-9.889 9.884a9.86 9.86 0 0 0 1.51 5.26l-.999 3.648 3.978-1.607zm11.387-5.464c-.074-.124-.272-.198-.57-.347-.297-.149-1.758-.868-2.031-.967-.272-.099-.47-.149-.669.149-.198.297-.768.967-.941 1.165-.173.198-.347.223-.644.074-.297-.149-1.255-.462-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.297-.347.446-.521.151-.172.2-.296.3-.495.099-.198.05-.372-.025-.521-.075-.148-.669-1.611-.916-2.206-.242-.579-.487-.501-.669-.51l-.57-.01c-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.263.489 1.694.626.712.226 1.36.194 1.872.118.571-.085 1.758-.719 2.006-1.413.248-.695.248-1.29.173-1.414z" />
  </Solido>
)

export const IconoDinero = (p) => (
  <Linea {...p}>
    <rect x="2" y="6" width="20" height="12" rx="2" />
    <circle cx="12" cy="12" r="2.5" />
    <path d="M6 9v6M18 9v6" />
  </Linea>
)

export const IconoMessenger = (p) => (
  <Solido {...p}>
    <path d="M12 0C5.24 0 0 4.952 0 11.64c0 3.499 1.434 6.522 3.769 8.61.196.176.314.42.322.681l.065 2.135a.96.96 0 0 0 1.347.84l2.381-1.05a.96.96 0 0 1 .642-.046c1.095.301 2.26.464 3.474.464 6.76 0 12-4.952 12-11.64C24 4.952 18.76 0 12 0zm7.2 8.94l-3.522 5.59a1.8 1.8 0 0 1-2.602.48l-2.802-2.099a.72.72 0 0 0-.866.002l-3.78 2.87c-.504.382-1.163-.222-.825-.755l3.522-5.59a1.8 1.8 0 0 1 2.602-.48l2.801 2.098a.72.72 0 0 0 .867-.001l3.78-2.871c.504-.383 1.163.221.825.755z" />
  </Solido>
)
