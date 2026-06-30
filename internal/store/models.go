package store

// Modelos del dominio. Las etiquetas JSON usan snake_case para que el frontend
// React (ya existente) funcione sin cambios.

type TipoEntrada struct {
	Nombre string  `json:"nombre"`
	Precio float64 `json:"precio"`
}

type Evento struct {
	ID             int64         `json:"id"`
	Nombre         string        `json:"nombre"`
	FechaHora      string        `json:"fecha_hora"`
	Lugar          string        `json:"lugar"`
	Aforo          *int          `json:"aforo"`
	TiposDeEntrada []TipoEntrada `json:"tipos_de_entrada"`
	Finalizado     bool          `json:"finalizado"` // archivado manualmente
	Imagen         *string       `json:"imagen"`     // nombre de archivo del afiche (en MEDIA_DIR); nil si no tiene
	Estado         string        `json:"estado"`     // derivado: Próximo | Finalizado
	Vendidas       int           `json:"vendidas"`
}

type Entrada struct {
	ID              int64   `json:"id"`
	EventoID        int64   `json:"evento_id"`
	NombreComprador string  `json:"nombre_comprador"`
	Contacto        *string `json:"contacto"`
	TipoEntrada     string  `json:"tipo_entrada"`
	Precio          float64 `json:"precio"`
	MedioPago       string  `json:"medio_pago"`
	EstadoPago      string  `json:"estado_pago"`
	CodigoQR        string  `json:"codigo_qr"`
	EstadoCheckin   string  `json:"estado_checkin"`
	HoraIngreso     *string `json:"hora_ingreso"`
	FechaRegistro   string  `json:"fecha_registro"`
}

// EventoMini son los datos del evento que acompañan a la vista pública de una entrada.
type EventoMini struct {
	Nombre    string `json:"nombre"`
	FechaHora string `json:"fecha_hora"`
	Lugar     string `json:"lugar"`
}

// EntradaPublica es una entrada con los datos de su evento (para compartir por WhatsApp).
type EntradaPublica struct {
	Entrada
	Evento EventoMini `json:"evento"`
}

// Resumen económico y de aforo de un evento.
type Resumen struct {
	EventoID          int64              `json:"evento_id"`
	Aforo             *int               `json:"aforo"`
	Vendidas          int                `json:"vendidas"`
	Disponibles       *int               `json:"disponibles"`
	Usadas            int                `json:"usadas"`
	Cobrado           float64            `json:"cobrado"`
	Pendiente         float64            `json:"pendiente"`
	RecaudadoPorMedio map[string]float64 `json:"recaudado_por_medio"`
}

// Usuario (administradora).
type Usuario struct {
	ID       int64
	Usuario  string
	Password string
}
