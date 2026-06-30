package store

// Resumen calcula las métricas económicas y de aforo de un evento.
func (s *Store) Resumen(eventoID int64) (*Resumen, error) {
	ev, err := s.ObtenerEvento(eventoID)
	if err != nil {
		return nil, err
	}

	rows, err := s.db.Query(
		"SELECT precio, medio_pago, estado_pago, estado_checkin FROM entradas WHERE evento_id = ?", eventoID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	r := &Resumen{
		EventoID:          ev.ID,
		Aforo:             ev.Aforo,
		RecaudadoPorMedio: map[string]float64{"Yape": 0, "Efectivo": 0},
	}
	for rows.Next() {
		var precio float64
		var medio, estadoPago, estadoCheckin string
		if err := rows.Scan(&precio, &medio, &estadoPago, &estadoCheckin); err != nil {
			return nil, err
		}
		r.Vendidas++
		if estadoPago == "Pagado" {
			r.Cobrado += precio
			r.RecaudadoPorMedio[medio] += precio
		} else {
			r.Pendiente += precio
		}
		if estadoCheckin == "Usada" {
			r.Usadas++
		}
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}

	if ev.Aforo != nil {
		disp := *ev.Aforo - r.Vendidas
		if disp < 0 {
			disp = 0
		}
		r.Disponibles = &disp
	}
	return r, nil
}
