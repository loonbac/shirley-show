package api

import "net/http"

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
	res, err := s.st.Checkin(eventoID, in.Codigo)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, res)
	return nil
}
