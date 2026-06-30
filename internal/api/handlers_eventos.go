package api

import (
	"net/http"
	"strings"

	"shirleyshow/internal/store"
)

type eventoInput struct {
	Nombre         string              `json:"nombre"`
	FechaHora      string              `json:"fecha_hora"`
	Lugar          string              `json:"lugar"`
	Aforo          *int                `json:"aforo"`
	TiposDeEntrada []store.TipoEntrada `json:"tipos_de_entrada"`
}

// validar revisa y normaliza la entrada del formulario de evento.
func (in eventoInput) aData() (store.EventoData, error) {
	nombre := strings.TrimSpace(in.Nombre)
	lugar := strings.TrimSpace(in.Lugar)
	if nombre == "" {
		return store.EventoData{}, errDatos("El nombre es obligatorio")
	}
	if in.FechaHora == "" {
		return store.EventoData{}, errDatos("La fecha y hora son obligatorias")
	}
	if lugar == "" {
		return store.EventoData{}, errDatos("El lugar es obligatorio")
	}
	if len(in.TiposDeEntrada) == 0 {
		return store.EventoData{}, errDatos("Agrega al menos un tipo de entrada")
	}
	tipos := make([]store.TipoEntrada, 0, len(in.TiposDeEntrada))
	for _, t := range in.TiposDeEntrada {
		n := strings.TrimSpace(t.Nombre)
		if n == "" || t.Precio < 0 {
			return store.EventoData{}, errDatos("Cada tipo de entrada necesita nombre y precio válido")
		}
		tipos = append(tipos, store.TipoEntrada{Nombre: n, Precio: t.Precio})
	}
	if in.Aforo != nil && *in.Aforo == 0 {
		in.Aforo = nil
	}
	return store.EventoData{Nombre: nombre, FechaHora: in.FechaHora, Lugar: lugar, Aforo: in.Aforo, Tipos: tipos}, nil
}

func (s *Server) listarEventos(w http.ResponseWriter, r *http.Request) error {
	lista, err := s.st.ListarEventos()
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, lista)
	return nil
}

func (s *Server) obtenerEvento(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	ev, err := s.st.ObtenerEvento(id)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, ev)
	return nil
}

func (s *Server) crearEvento(w http.ResponseWriter, r *http.Request) error {
	var in eventoInput
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	data, err := in.aData()
	if err != nil {
		return err
	}
	ev, err := s.st.CrearEvento(data)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusCreated, ev)
	return nil
}

func (s *Server) actualizarEvento(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	var in eventoInput
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	data, err := in.aData()
	if err != nil {
		return err
	}
	ev, err := s.st.ActualizarEvento(id, data)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, ev)
	return nil
}

func (s *Server) finalizarEvento(w http.ResponseWriter, r *http.Request) error {
	return s.cambiarFinalizado(w, r, true)
}

func (s *Server) reabrirEvento(w http.ResponseWriter, r *http.Request) error {
	return s.cambiarFinalizado(w, r, false)
}

func (s *Server) cambiarFinalizado(w http.ResponseWriter, r *http.Request, finalizado bool) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	ev, err := s.st.MarcarFinalizado(id, finalizado)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, ev)
	return nil
}

func (s *Server) eliminarEvento(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	ev, _ := s.st.ObtenerEvento(id) // para borrar su afiche tras eliminar
	if err := s.st.EliminarEvento(id); err != nil {
		return err
	}
	s.borrarImagenDeEvento(ev)
	escribirJSON(w, http.StatusOK, map[string]bool{"eliminado": true})
	return nil
}

func (s *Server) resumen(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	res, err := s.st.Resumen(id)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, res)
	return nil
}
