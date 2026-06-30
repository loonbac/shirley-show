package api

import (
	"fmt"
	"net/http"
	"strings"
)

func (s *Server) whatsappEstado(w http.ResponseWriter, r *http.Request) error {
	if s.wa == nil {
		return errDatos("WhatsApp no está activado en el servidor")
	}
	escribirJSON(w, http.StatusOK, s.wa.Estado())
	return nil
}

func (s *Server) whatsappConectar(w http.ResponseWriter, r *http.Request) error {
	if s.wa == nil {
		return errDatos("WhatsApp no está activado en el servidor")
	}
	if err := s.wa.Conectar(); err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, s.wa.Estado())
	return nil
}

func (s *Server) enviarWhatsApp(w http.ResponseWriter, r *http.Request) error {
	if s.wa == nil {
		return errDatos("WhatsApp no está activado en el servidor")
	}
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	e, err := s.st.ObtenerEntrada(id)
	if err != nil {
		return err
	}
	if e.Contacto == nil || *e.Contacto == "" {
		return errDatos("La entrada no tiene un contacto para enviar")
	}
	link := s.linkEntrada(r, e.CodigoQR)
	msg := fmt.Sprintf("¡Hola %s! Aquí está tu entrada para el evento. Muéstrala en la puerta: %s", e.NombreComprador, link)
	if err := s.wa.EnviarTexto(*e.Contacto, msg); err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, map[string]bool{"enviado": true})
	return nil
}

// linkEntrada arma el enlace público de la entrada. Si PUBLIC_BASE_URL está configurado
// lo usa (evita Host header injection: el enlace no depende de la cabecera Host del
// cliente). Si no, cae al host y esquema reales de la petición (https tras el túnel).
func (s *Server) linkEntrada(r *http.Request, codigo string) string {
	if base := s.cfg.PublicBaseURL; base != "" {
		return fmt.Sprintf("%s/entrada/%s", strings.TrimRight(base, "/"), codigo)
	}
	esquema := "http"
	if esHTTPS(r) {
		esquema = "https"
	}
	return fmt.Sprintf("%s://%s/entrada/%s", esquema, r.Host, codigo)
}
