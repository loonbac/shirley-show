package api

import (
	"net/http"
	"strings"

	"shirleyshow/internal/store"
)

// categoriasGasto son las categorías permitidas (deben coincidir con el frontend).
var categoriasGasto = map[string]bool{
	"Bolo de artistas":  true,
	"Pasajes":           true,
	"Comida":            true,
	"Alquiler de local": true,
	"Otro":              true,
}

type gastoInput struct {
	Categoria   string  `json:"categoria"`
	Descripcion *string `json:"descripcion"`
	Monto       float64 `json:"monto"`
}

// aData valida y normaliza la entrada del formulario de gasto.
func (in gastoInput) aData(eventoID int64) (store.GastoData, error) {
	cat := strings.TrimSpace(in.Categoria)
	if !categoriasGasto[cat] {
		return store.GastoData{}, errDatos("Elige una categoría de gasto válida")
	}
	if in.Monto <= 0 {
		return store.GastoData{}, errDatos("El monto debe ser mayor que cero")
	}
	if in.Monto > 10_000_000 {
		return store.GastoData{}, errDatos("El monto es demasiado alto")
	}
	return store.GastoData{EventoID: eventoID, Categoria: cat, Descripcion: in.Descripcion, Monto: in.Monto}, nil
}

func (s *Server) listarGastos(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	lista, err := s.st.ListarGastos(id)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, lista)
	return nil
}

func (s *Server) crearGasto(w http.ResponseWriter, r *http.Request) error {
	eventoID, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	if _, err := s.st.ObtenerEvento(eventoID); err != nil {
		return err
	}
	var in gastoInput
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	data, err := in.aData(eventoID)
	if err != nil {
		return err
	}
	g, err := s.st.CrearGasto(data)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusCreated, g)
	return nil
}

func (s *Server) actualizarGasto(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	var in gastoInput
	if err := leerJSON(r, &in); err != nil {
		return err
	}
	data, err := in.aData(0) // el evento no cambia al editar
	if err != nil {
		return err
	}
	g, err := s.st.ActualizarGasto(id, data)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, g)
	return nil
}

func (s *Server) eliminarGasto(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	if err := s.st.EliminarGasto(id); err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, map[string]bool{"eliminado": true})
	return nil
}
