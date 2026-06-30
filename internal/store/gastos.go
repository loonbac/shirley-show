package store

import (
	"database/sql"
	"errors"
	"time"
)

// GastoData son los datos (ya validados) para crear o actualizar un gasto.
type GastoData struct {
	EventoID    int64
	Categoria   string
	Descripcion *string
	Monto       float64
}

const colsGasto = `id, evento_id, categoria, descripcion, monto, fecha_creacion`

func escanearGasto(row interface{ Scan(...any) error }) (*Gasto, error) {
	var g Gasto
	var descripcion sql.NullString
	if err := row.Scan(&g.ID, &g.EventoID, &g.Categoria, &descripcion, &g.Monto, &g.FechaCreacion); err != nil {
		return nil, err
	}
	if descripcion.Valid {
		g.Descripcion = &descripcion.String
	}
	return &g, nil
}

func (s *Store) ListarGastos(eventoID int64) ([]Gasto, error) {
	rows, err := s.db.Query("SELECT "+colsGasto+
		" FROM gastos WHERE evento_id = ? ORDER BY fecha_creacion DESC, id DESC", eventoID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	lista := []Gasto{}
	for rows.Next() {
		g, err := escanearGasto(rows)
		if err != nil {
			return nil, err
		}
		lista = append(lista, *g)
	}
	return lista, rows.Err()
}

func (s *Store) ObtenerGasto(id int64) (*Gasto, error) {
	g, err := escanearGasto(s.db.QueryRow("SELECT "+colsGasto+" FROM gastos WHERE id = ?", id))
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNoEncontrado
	}
	return g, err
}

func (s *Store) CrearGasto(d GastoData) (*Gasto, error) {
	ahora := time.Now().Format(time.RFC3339)
	res, err := s.db.Exec(
		"INSERT INTO gastos (evento_id, categoria, descripcion, monto, fecha_creacion) VALUES (?,?,?,?,?)",
		d.EventoID, d.Categoria, contactoSQL(d.Descripcion), d.Monto, ahora)
	if err != nil {
		return nil, err
	}
	id, _ := res.LastInsertId()
	return s.ObtenerGasto(id)
}

func (s *Store) ActualizarGasto(id int64, d GastoData) (*Gasto, error) {
	if _, err := s.ObtenerGasto(id); err != nil {
		return nil, err
	}
	_, err := s.db.Exec("UPDATE gastos SET categoria = ?, descripcion = ?, monto = ? WHERE id = ?",
		d.Categoria, contactoSQL(d.Descripcion), d.Monto, id)
	if err != nil {
		return nil, err
	}
	return s.ObtenerGasto(id)
}

func (s *Store) EliminarGasto(id int64) error {
	res, err := s.db.Exec("DELETE FROM gastos WHERE id = ?", id)
	if err != nil {
		return err
	}
	if n, _ := res.RowsAffected(); n == 0 {
		return ErrNoEncontrado
	}
	return nil
}

// TotalGastos suma todos los gastos de un evento (lo usa el resumen).
func (s *Store) TotalGastos(eventoID int64) (float64, error) {
	var total sql.NullFloat64
	err := s.db.QueryRow("SELECT SUM(monto) FROM gastos WHERE evento_id = ?", eventoID).Scan(&total)
	if err != nil {
		return 0, err
	}
	if !total.Valid {
		return 0, nil
	}
	return total.Float64, nil
}
