package api

import (
	"net/http"
	"strings"

	"shirleyshow/internal/store"
)

var mediosPago = map[string]bool{"Yape": true, "Efectivo": true}

func (s *Server) listarEntradas(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	lista, err := s.st.ListarEntradas(id, r.URL.Query().Get("estado_pago"), r.URL.Query().Get("q"))
	if err != nil {
		return err
	}
	for i := range lista {
		s.ponerQR(r, &lista[i])
	}
	escribirJSON(w, http.StatusOK, lista)
	return nil
}

func (s *Server) registrarVenta(w http.ResponseWriter, r *http.Request) error {
	eventoID, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	var in struct {
		NombreComprador string  `json:"nombre_comprador"`
		Contacto        *string `json:"contacto"`
		TipoEntrada     string  `json:"tipo_entrada"`
		Cantidad        int     `json:"cantidad"`
		MedioPago       string  `json:"medio_pago"`
		EstadoPago      string  `json:"estado_pago"`
		Cortesia        bool    `json:"cortesia"` // entrada gratis (promotora que cumplió): S/ 0 y QR normal
	}
	if err := leerJSON(r, &in); err != nil {
		return err
	}

	evento, err := s.st.ObtenerEvento(eventoID)
	if err != nil {
		return err
	}
	if strings.TrimSpace(in.NombreComprador) == "" {
		return errDatos("El nombre del comprador es obligatorio")
	}
	var precio float64
	hallado := false
	for _, t := range evento.TiposDeEntrada {
		if t.Nombre == in.TipoEntrada {
			precio, hallado = t.Precio, true
			break
		}
	}
	if !hallado {
		return errDatos("Tipo de entrada inválido para este evento")
	}
	if in.Cantidad < 1 || in.Cantidad > 50 {
		return errDatos("Cantidad inválida (1 a 50)")
	}
	medioPago := in.MedioPago
	estadoPago := "Pendiente"
	if in.EstadoPago == "Pagado" {
		estadoPago = "Pagado"
	}
	if in.Cortesia {
		// La cortesía no se cobra: queda "Pagado" para no figurar como deuda pendiente.
		precio, medioPago, estadoPago = 0, "Cortesía", "Pagado"
	} else if !mediosPago[in.MedioPago] {
		return errDatos("Medio de pago inválido")
	}

	creadas, err := s.st.CrearEntradas(store.VentaData{
		EventoID:        eventoID,
		NombreComprador: strings.TrimSpace(in.NombreComprador),
		Contacto:        in.Contacto,
		TipoEntrada:     in.TipoEntrada,
		Precio:          precio,
		MedioPago:       medioPago,
		EstadoPago:      estadoPago,
		Cantidad:        in.Cantidad,
	})
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusCreated, creadas)
	return nil
}

func (s *Server) obtenerEntrada(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	e, err := s.st.ObtenerEntrada(id)
	if err != nil {
		return err
	}
	s.ponerQR(r, e)
	escribirJSON(w, http.StatusOK, e)
	return nil
}

func (s *Server) confirmarPago(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	e, err := s.st.ConfirmarPago(id)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, e)
	return nil
}

func (s *Server) entradaPublica(w http.ResponseWriter, r *http.Request) error {
	e, err := s.st.ObtenerEntradaPorCodigo(r.PathValue("codigo"))
	if err != nil {
		return err
	}
	s.ponerQR(r, &e.Entrada)
	escribirJSON(w, http.StatusOK, e)
	return nil
}
