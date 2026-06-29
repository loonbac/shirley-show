package store

import (
	"database/sql"
	"encoding/json"
	"errors"
	"time"
)

// ErrNoEncontrado indica que un registro no existe.
var ErrNoEncontrado = errors.New("no encontrado")

// EventoData son los datos de entrada para crear o actualizar un evento (ya validados).
type EventoData struct {
	Nombre    string
	FechaHora string
	Lugar     string
	Aforo     *int
	Tipos     []TipoEntrada
}

// deriveEstado calcula Próximo/Finalizado según la fecha del evento.
func deriveEstado(fechaHora string) string {
	t, err := time.Parse(time.RFC3339, fechaHora)
	if err == nil && t.Before(time.Now()) {
		return "Finalizado"
	}
	return "Próximo"
}

func escanearEvento(row interface{ Scan(...any) error }) (*Evento, error) {
	var e Evento
	var aforo sql.NullInt64
	var tiposJSON string
	if err := row.Scan(&e.ID, &e.Nombre, &e.FechaHora, &e.Lugar, &aforo, &tiposJSON, &e.Vendidas); err != nil {
		return nil, err
	}
	if aforo.Valid {
		v := int(aforo.Int64)
		e.Aforo = &v
	}
	_ = json.Unmarshal([]byte(tiposJSON), &e.TiposDeEntrada)
	if e.TiposDeEntrada == nil {
		e.TiposDeEntrada = []TipoEntrada{}
	}
	e.Estado = deriveEstado(e.FechaHora)
	return &e, nil
}

const selEvento = `SELECT e.id, e.nombre, e.fecha_hora, e.lugar, e.aforo, e.tipos_de_entrada,
	(SELECT COUNT(*) FROM entradas WHERE evento_id = e.id) AS vendidas FROM eventos e`

func (s *Store) ListarEventos() ([]Evento, error) {
	rows, err := s.db.Query(selEvento + " ORDER BY e.fecha_hora DESC")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	lista := []Evento{}
	for rows.Next() {
		e, err := escanearEvento(rows)
		if err != nil {
			return nil, err
		}
		lista = append(lista, *e)
	}
	return lista, rows.Err()
}

func (s *Store) ObtenerEvento(id int64) (*Evento, error) {
	e, err := escanearEvento(s.db.QueryRow(selEvento+" WHERE e.id = ?", id))
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNoEncontrado
	}
	return e, err
}

func (s *Store) CrearEvento(d EventoData) (*Evento, error) {
	tipos, _ := json.Marshal(d.Tipos)
	res, err := s.db.Exec(
		`INSERT INTO eventos (nombre, fecha_hora, lugar, aforo, tipos_de_entrada, fecha_creacion) VALUES (?,?,?,?,?,?)`,
		d.Nombre, d.FechaHora, d.Lugar, aforoSQL(d.Aforo), string(tipos), time.Now().Format(time.RFC3339),
	)
	if err != nil {
		return nil, err
	}
	id, _ := res.LastInsertId()
	return s.ObtenerEvento(id)
}

func (s *Store) ActualizarEvento(id int64, d EventoData) (*Evento, error) {
	if _, err := s.ObtenerEvento(id); err != nil {
		return nil, err
	}
	tipos, _ := json.Marshal(d.Tipos)
	_, err := s.db.Exec(
		`UPDATE eventos SET nombre=?, fecha_hora=?, lugar=?, aforo=?, tipos_de_entrada=? WHERE id=?`,
		d.Nombre, d.FechaHora, d.Lugar, aforoSQL(d.Aforo), string(tipos), id,
	)
	if err != nil {
		return nil, err
	}
	return s.ObtenerEvento(id)
}

func aforoSQL(a *int) any {
	if a == nil {
		return nil
	}
	return *a
}
