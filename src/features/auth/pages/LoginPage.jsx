import { LoginForm } from '../components/LoginForm.jsx'
import { Tarjeta } from '../../../components/Tarjeta.jsx'
import './LoginPage.css'

// Pantalla de login. Mobile-first, con el logo de Shirley Show.
export default function LoginPage() {
  return (
    <div className="login">
      <div className="login__marca">
        <img className="login__logo" src="/logo.png" alt="Shirley Show y sus amigos" />
        <p className="login__sub">Gestión de eventos</p>
      </div>
      <Tarjeta>
        <LoginForm />
      </Tarjeta>
    </div>
  )
}
