package api

import (
	"net/http"
	"strings"
)

func (s *Server) checkin(w http.ResponseWriter, r *http.Request) error {
	eventoID, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	var in struct {
		Codigo string `json:"codigo"`
	}
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	res, err := s.st.Checkin(eventoID, s.resolverCodigoEscaneado(in.Codigo))
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, res)
	return nil
}

// resolverCodigoEscaneado convierte lo leído por el escáner al código real de la entrada.
// El QR nuevo es una URL .../r/<token>: extrae el token y lo resuelve contra la base.
// Si es el código pelado (o no se resuelve), lo devuelve tal cual (ingreso manual / compat).
func (s *Server) resolverCodigoEscaneado(scan string) string {
	scan = strings.TrimSpace(scan)
	if i := strings.Index(scan, "/r/"); i >= 0 {
		tok := scan[i+3:]
		if j := strings.IndexAny(tok, "/?#"); j >= 0 {
			tok = tok[:j]
		}
		if codigo, err := s.st.CodigoPorToken(tok); err == nil && codigo != "" {
			return codigo
		}
	}
	return scan
}
