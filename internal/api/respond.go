package api

import (
	"encoding/json"
	"net/http"
	"strconv"
)

// apiError lleva un código HTTP junto al mensaje (errores de negocio).
type apiError struct {
	status int
	msg    string
}

func (e apiError) Error() string { return e.msg }

func errDatos(msg string) error     { return apiError{http.StatusBadRequest, msg} }
func errNoAutorizado(msg string) error { return apiError{http.StatusUnauthorized, msg} }

func escribirJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

func errorJSON(w http.ResponseWriter, status int, msg string) {
	escribirJSON(w, status, map[string]string{"error": msg})
}

// leerJSON decodifica el cuerpo de la petición en dst.
func leerJSON(r *http.Request, dst any) error {
	if err := json.NewDecoder(r.Body).Decode(dst); err != nil {
		return errDatos("JSON inválido en la petición")
	}
	return nil
}

// idDeRuta lee un parámetro numérico de la ruta (ej. {id}).
func idDeRuta(r *http.Request, nombre string) (int64, error) {
	id, err := strconv.ParseInt(r.PathValue(nombre), 10, 64)
	if err != nil {
		return 0, errDatos("Identificador inválido")
	}
	return id, nil
}
