package api

import (
	"net/http"
	"strings"

	"shirleyshow/internal/auth"
)

func (s *Server) login(w http.ResponseWriter, r *http.Request) error {
	var in struct {
		Usuario  string `json:"usuario"`
		Password string `json:"password"`
	}
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	u, err := s.st.ObtenerUsuario(strings.TrimSpace(in.Usuario))
	if err != nil || !auth.Verificar(u.Password, in.Password) {
		return errNoAutorizado("Usuario o contraseña incorrectos")
	}
	token, err := auth.FirmarToken(s.cfg.JWTSecret, u.ID, u.Usuario)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, map[string]string{"token": token, "usuario": u.Usuario})
	return nil
}
