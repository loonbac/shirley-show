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

// lineaVenta es "cuántas entradas de cada tipo" dentro de una misma venta
// (ej. 2 General + 1 VIP para la misma compradora).
type lineaVenta struct {
	TipoEntrada string `json:"tipo_entrada"`
	Cantidad    int    `json:"cantidad"`
}

func (s *Server) registrarVenta(w http.ResponseWriter, r *http.Request) error {
	eventoID, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	var in struct {
		NombreComprador string       `json:"nombre_comprador"`
		Contacto        *string      `json:"contacto"`
		Lineas          []lineaVenta `json:"lineas"`
		TipoEntrada     string       `json:"tipo_entrada"` // forma antigua: un solo tipo
		Cantidad        int          `json:"cantidad"`
		MedioPago       string       `json:"medio_pago"`
		EstadoPago      string       `json:"estado_pago"`
		Cortesia        bool         `json:"cortesia"` // entrada gratis (promotora que cumplió): S/ 0 y QR normal
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
	lineas := in.Lineas
	if len(lineas) == 0 {
		lineas = []lineaVenta{{TipoEntrada: in.TipoEntrada, Cantidad: in.Cantidad}}
	}
	precios := map[string]float64{}
	for _, t := range evento.TiposDeEntrada {
		precios[t.Nombre] = t.Precio
	}
	total := 0
	for _, l := range lineas {
		if _, ok := precios[l.TipoEntrada]; !ok {
			return errDatos("Tipo de entrada inválido para este evento")
		}
		if l.Cantidad < 1 || l.Cantidad > 50 {
			return errDatos("Cantidad inválida (1 a 50)")
		}
		total += l.Cantidad
	}
	if total > 50 {
		return errDatos("Máximo 50 entradas por venta")
	}
	medioPago := in.MedioPago
	estadoPago := "Pendiente"
	if in.EstadoPago == "Pagado" {
		estadoPago = "Pagado"
	}
	if in.Cortesia {
		// La cortesía no se cobra: queda "Pagado" para no figurar como deuda pendiente.
		medioPago, estadoPago = "Cortesía", "Pagado"
	} else if !mediosPago[in.MedioPago] {
		return errDatos("Medio de pago inválido")
	}

	creadas := []store.Entrada{}
	for _, l := range lineas {
		precio := precios[l.TipoEntrada]
		if in.Cortesia {
			precio = 0
		}
		nuevas, err := s.st.CrearEntradas(store.VentaData{
			EventoID:        eventoID,
			NombreComprador: strings.TrimSpace(in.NombreComprador),
			Contacto:        in.Contacto,
			TipoEntrada:     l.TipoEntrada,
			Precio:          precio,
			MedioPago:       medioPago,
			EstadoPago:      estadoPago,
			Cantidad:        l.Cantidad,
		})
		if err != nil {
			return err
		}
		creadas = append(creadas, nuevas...)
	}
	escribirJSON(w, http.StatusCreated, creadas)
	return nil
}

// eliminarEntrada borra una entrada registrada por error; su QR deja de valer.
func (s *Server) eliminarEntrada(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	if err := s.st.EliminarEntrada(id); err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, map[string]bool{"eliminado": true})
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
