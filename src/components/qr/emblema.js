// Emblema musical central del QR (estilo "logo grande" como los QR de marca).
// Insignia con degradado de marca y un par de corcheas unidas (♫) en blanco.
// Se devuelve como data URL para incrustarlo en el centro del QR.

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <defs>
    <linearGradient id="ge" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#7c3aed"/>
      <stop offset="1" stop-color="#c026d3"/>
    </linearGradient>
  </defs>
  <rect x="4" y="4" width="92" height="92" rx="26" fill="#ffffff"/>
  <rect x="8" y="8" width="84" height="84" rx="23" fill="url(#ge)"/>
  <g fill="#ffffff">
    <ellipse cx="33" cy="69" rx="11" ry="9" transform="rotate(-18 33 69)"/>
    <ellipse cx="64" cy="75" rx="11" ry="9" transform="rotate(-18 64 75)"/>
    <rect x="42" y="30" width="5" height="40" rx="2"/>
    <rect x="73" y="36" width="5" height="40" rx="2"/>
    <path d="M42 28 L78 34 L78 46 L42 40 Z"/>
  </g>
</svg>`

export const emblemaDataUrl = `data:image/svg+xml,${encodeURIComponent(svg.trim())}`
