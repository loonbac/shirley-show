import { LoginForm } from '../components/LoginForm.jsx'
import { Tarjeta } from '../../../components/Tarjeta.jsx'
import { IconoNota } from '../../../components/iconos/Iconos.jsx'
import './LoginPage.css'

// Pantalla de login. Mobile-first, con la marca Shirley Show.
export default function LoginPage() {
  return (
    <div className="login">
      <div className="login__marca">
        <span className="login__notas">
          <IconoNota size={30} />
          <IconoNota size={30} />
        </span>
        <h1 className="login__titulo">Shirley Show</h1>
        <p className="login__sub">Gestión de eventos</p>
      </div>
      <Tarjeta>
        <LoginForm />
      </Tarjeta>
    </div>
  )
}
