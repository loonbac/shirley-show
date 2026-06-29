package store

import (
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"errors"
	"strings"
	"time"
)

// generarCodigoQR crea un código único e irrepetible (16 bytes -> 32 hex, no adivinable).
func generarCodigoQR() string {
	b := make([]byte, 16)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}

// GenerarCodigoQR expone el generador de códigos (lo usa el seed).
func GenerarCodigoQR() string { return generarCodigoQR() }

// VentaData son los datos (ya validados) para registrar una venta de N entradas.
type VentaData struct {
	EventoID        int64
	NombreComprador string
	Contacto        *string
	TipoEntrada     string
	Precio          float64
	MedioPago       string
	EstadoPago      string
	Cantidad        int
}

const colsEntrada = `id, evento_id, nombre_comprador, contacto, tipo_entrada, precio,
	medio_pago, estado_pago, codigo_qr, estado_checkin, hora_ingreso, fecha_registro`

func escanearEntrada(row interface{ Scan(...any) error }) (*Entrada, error) {
	var e Entrada
	var contacto, horaIngreso sql.NullString
	err := row.Scan(&e.ID, &e.EventoID, &e.NombreComprador, &contacto, &e.TipoEntrada, &e.Precio,
		&e.MedioPago, &e.EstadoPago, &e.CodigoQR, &e.EstadoCheckin, &horaIngreso, &e.FechaRegistro)
	if err != nil {
		return nil, err
	}
	if contacto.Valid {
		e.Contacto = &contacto.String
	}
	if horaIngreso.Valid {
		e.HoraIngreso = &horaIngreso.String
	}
	return &e, nil
}

func (s *Store) ListarEntradas(eventoID int64, estadoPago, q string) ([]Entrada, error) {
	sqlStr := "SELECT " + colsEntrada + " FROM entradas WHERE evento_id = ?"
	args := []any{eventoID}
	if estadoPago == "Pagado" || estadoPago == "Pendiente" {
		sqlStr += " AND estado_pago = ?"
		args = append(args, estadoPago)
	}
	if t := strings.TrimSpace(q); t != "" {
		sqlStr += " AND LOWER(nombre_comprador) LIKE ?"
		args = append(args, "%"+strings.ToLower(t)+"%")
	}
	sqlStr += " ORDER BY fecha_registro DESC, id DESC"
	rows, err := s.db.Query(sqlStr, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	lista := []Entrada{}
	for rows.Next() {
		e, err := escanearEntrada(rows)
		if err != nil {
			return nil, err
		}
		lista = append(lista, *e)
	}
	return lista, rows.Err()
}

func (s *Store) ObtenerEntrada(id int64) (*Entrada, error) {
	e, err := escanearEntrada(s.db.QueryRow("SELECT "+colsEntrada+" FROM entradas WHERE id = ?", id))
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNoEncontrado
	}
	return e, err
}

func (s *Store) ObtenerEntradaPorCodigo(codigo string) (*EntradaPublica, error) {
	e, err := escanearEntrada(s.db.QueryRow("SELECT "+colsEntrada+" FROM entradas WHERE codigo_qr = ?", strings.TrimSpace(codigo)))
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNoEncontrado
	}
	if err != nil {
		return nil, err
	}
	var ev EventoMini
	_ = s.db.QueryRow("SELECT nombre, fecha_hora, lugar FROM eventos WHERE id = ?", e.EventoID).
		Scan(&ev.Nombre, &ev.FechaHora, &ev.Lugar)
	return &EntradaPublica{Entrada: *e, Evento: ev}, nil
}

// CrearEntradas genera `Cantidad` entradas individuales, cada una con su QR único.
func (s *Store) CrearEntradas(d VentaData) ([]Entrada, error) {
	ahora := time.Now().Format(time.RFC3339)
	stmt, err := s.db.Prepare(`INSERT INTO entradas
		(evento_id, nombre_comprador, contacto, tipo_entrada, precio, medio_pago, estado_pago, codigo_qr, estado_checkin, fecha_registro)
		VALUES (?,?,?,?,?,?,?,?, 'Pendiente', ?)`)
	if err != nil {
		return nil, err
	}
	defer stmt.Close()

	creadas := make([]Entrada, 0, d.Cantidad)
	for i := 0; i < d.Cantidad; i++ {
		res, err := stmt.Exec(d.EventoID, d.NombreComprador, contactoSQL(d.Contacto), d.TipoEntrada,
			d.Precio, d.MedioPago, d.EstadoPago, generarCodigoQR(), ahora)
		if err != nil {
			return nil, err
		}
		id, _ := res.LastInsertId()
		e, err := s.ObtenerEntrada(id)
		if err != nil {
			return nil, err
		}
		creadas = append(creadas, *e)
	}
	return creadas, nil
}

func (s *Store) ConfirmarPago(id int64) (*Entrada, error) {
	if _, err := s.ObtenerEntrada(id); err != nil {
		return nil, err
	}
	if _, err := s.db.Exec("UPDATE entradas SET estado_pago = 'Pagado' WHERE id = ?", id); err != nil {
		return nil, err
	}
	return s.ObtenerEntrada(id)
}

func contactoSQL(c *string) any {
	if c == nil || strings.TrimSpace(*c) == "" {
		return nil
	}
	return strings.TrimSpace(*c)
}
