package api

import (
	"log"
	"net/http"
	"strings"

	"shirleyshow/internal/auth"
)

func (s *Server) login(w http.ResponseWriter, r *http.Request) error {
	ip := ipCliente(r, s.cfg.TrustedProxies)
	// Fuerza bruta: bloquea tras varios intentos fallidos desde la misma IP.
	if !s.limiter.permitido(ip) {
		return errDemasiados("Demasiados intentos. Espera unos minutos e inténtalo de nuevo.")
	}

	var in struct {
		Usuario  string `json:"usuario"`
		Password string `json:"password"`
	}
	if err := leerJSON(r, &in); err != nil {
		return err
	}

	u, err := s.st.ObtenerUsuario(strings.TrimSpace(in.Usuario))
	if err != nil {
		auth.CompararFicticio(in.Password) // tiempo constante: no revelar si el usuario existe
		s.limiter.fallo(ip)
		log.Printf("login fallido (usuario inexistente) ip=%s usuario=%q", ip, in.Usuario)
		return errNoAutorizado("Usuario o contraseña incorrectos")
	}
	if !auth.Verificar(u.Password, in.Password) {
		s.limiter.fallo(ip)
		log.Printf("login fallido (clave incorrecta) ip=%s usuario=%q", ip, in.Usuario)
		return errNoAutorizado("Usuario o contraseña incorrectos")
	}

	s.limiter.exito(ip)
	token, err := auth.FirmarToken(s.cfg.JWTSecret, u.ID, u.Usuario)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, map[string]string{"token": token, "usuario": u.Usuario})
	return nil
}
