import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { useSesion } from '../hooks/useSesion.jsx'

// Formulario de inicio de sesión de la administradora.
export function LoginForm() {
  const { iniciarSesion } = useSesion()
  const navegar = useNavigate()
  const [usuario, setUsuario] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [cargando, setCargando] = useState(false)

  async function enviar(e) {
    e.preventDefault()
    setError('')
    setCargando(true)
    try {
      await iniciarSesion(usuario, password)
      navegar('/eventos')
    } catch (err) {
      setError(err.message)
    } finally {
      setCargando(false)
    }
  }

  return (
    <form className="pila" onSubmit={enviar}>
      <Campo etiqueta="Usuario" htmlFor="usuario">
        <Input id="usuario" value={usuario} onChange={(e) => setUsuario(e.target.value)} autoComplete="username" autoFocus />
      </Campo>
      <Campo etiqueta="Contraseña" htmlFor="password">
        <Input id="password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="current-password" />
      </Campo>
      {error && <MensajeError>{error}</MensajeError>}
      <Boton type="submit" ancho disabled={cargando}>
        {cargando ? 'Entrando…' : 'Entrar'}
      </Boton>
    </form>
  )
}
