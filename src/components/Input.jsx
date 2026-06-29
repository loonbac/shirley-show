// Input de texto genérico. Usa el estilo compartido `.control` (ver Campo.css).
export function Input({ className = '', ...props }) {
  return <input className={`control ${className}`} {...props} />
}
