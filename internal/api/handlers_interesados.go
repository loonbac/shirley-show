package api

import (
	"net/http"
	"strings"

	"shirleyshow/internal/store"
)

type interesadoInput struct {
	Nombre    string  `json:"nombre"`
	Contacto  *string `json:"contacto"`
	Canal     *string `json:"canal"`
	Personas  int     `json:"personas"`
	Promotora bool    `json:"promotora"`
}

// aData valida y normaliza la entrada del formulario de interesado.
func (in interesadoInput) aData(eventoID int64) (store.InteresadoData, error) {
	nombre := strings.TrimSpace(in.Nombre)
	if nombre == "" {
		return store.InteresadoData{}, errDatos("Escribe el nombre de la persona")
	}
	canal, err := canalValido(in.Canal)
	if err != nil {
		return store.InteresadoData{}, err
	}
	personas := in.Personas
	if personas < 1 {
		personas = 1 // por defecto viene una sola persona
	}
	if personas > 100 {
		return store.InteresadoData{}, errDatos("El número de personas es demasiado alto")
	}
	return store.InteresadoData{EventoID: eventoID, Nombre: nombre, Contacto: in.Contacto, Canal: canal, Personas: personas, Promotora: in.Promotora}, nil
}

// canalValido acepta solo los canales de contacto conocidos (o ninguno).
func canalValido(c *string) (*string, error) {
	if c == nil {
		return nil, nil
	}
	v := strings.TrimSpace(*c)
	switch v {
	case "":
		return nil, nil
	case "WhatsApp", "Messenger":
		return &v, nil
	default:
		return nil, errDatos("Canal de contacto inválido")
	}
}

func (s *Server) listarInteresados(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	lista, err := s.st.ListarInteresados(id)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, lista)
	return nil
}

func (s *Server) crearInteresado(w http.ResponseWriter, r *http.Request) error {
	eventoID, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	if _, err := s.st.ObtenerEvento(eventoID); err != nil {
		return err
	}
	var in interesadoInput
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	data, err := in.aData(eventoID)
	if err != nil {
		return err
	}
	it, err := s.st.CrearInteresado(data)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusCreated, it)
	return nil
}

func (s *Server) actualizarInteresado(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	var in interesadoInput
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	data, err := in.aData(0) // el evento no cambia al editar
	if err != nil {
		return err
	}
	it, err := s.st.ActualizarInteresado(id, data)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, it)
	return nil
}

func (s *Server) confirmarInteresado(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	it, err := s.st.MarcarInteresadoConfirmado(id)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, it)
	return nil
}

func (s *Server) eliminarInteresado(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	if err := s.st.EliminarInteresado(id); err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, map[string]bool{"eliminado": true})
	return nil
}
