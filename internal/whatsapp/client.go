// Package whatsapp integra WhatsApp (whatsmeow) para enviar entradas directamente.
package whatsapp

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"

	waE2E "go.mau.fi/whatsmeow/proto/waE2E"
	"go.mau.fi/whatsmeow/store/sqlstore"
	"go.mau.fi/whatsmeow/types"
	waLog "go.mau.fi/whatsmeow/util/log"
	"go.mau.fi/whatsmeow"
	"google.golang.org/protobuf/proto"
	_ "modernc.org/sqlite"
)

// Cliente envuelve el cliente de whatsmeow y guarda el último QR de emparejamiento.
type Cliente struct {
	wa       *whatsmeow.Client
	mu       sync.Mutex
	ultimoQR string
}

// Estado resume la situación de la conexión con WhatsApp.
type Estado struct {
	Conectado bool   `json:"conectado"`
	Vinculado bool   `json:"vinculado"`    // ya tiene sesión guardada
	QR        string `json:"qr,omitempty"` // texto para mostrar como QR al emparejar
}

// Nuevo abre la sesión de WhatsApp (SQLite propio) y conecta si ya estaba vinculada.
func Nuevo(rutaStore string) (*Cliente, error) {
	if err := os.MkdirAll(filepath.Dir(rutaStore), 0o755); err != nil {
		return nil, err
	}
	dsn := fmt.Sprintf("file:%s?_pragma=foreign_keys(1)&_pragma=busy_timeout(5000)", rutaStore)
	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, err
	}
	ctx := context.Background()
	// dialecto "sqlite3" (SQL) sobre el driver puro "sqlite" (modernc).
	container := sqlstore.NewWithDB(db, "sqlite3", waLog.Noop)
	if err := container.Upgrade(ctx); err != nil {
		return nil, err
	}
	device, err := container.GetFirstDevice(ctx)
	if err != nil {
		return nil, err
	}
	c := &Cliente{wa: whatsmeow.NewClient(device, waLog.Noop)}
	if c.wa.Store.ID != nil {
		if err := c.wa.Connect(); err != nil {
			return nil, err
		}
	}
	return c, nil
}

func (c *Cliente) Estado() Estado {
	c.mu.Lock()
	defer c.mu.Unlock()
	return Estado{
		Conectado: c.wa.IsConnected(),
		Vinculado: c.wa.Store.ID != nil,
		QR:        c.ultimoQR,
	}
}

// Conectar inicia el emparejamiento: si no hay sesión, genera un QR para escanear
// desde el WhatsApp de la cantante. Si ya está vinculada, solo asegura la conexión.
func (c *Cliente) Conectar() error {
	if c.wa.Store.ID != nil {
		if !c.wa.IsConnected() {
			return c.wa.Connect()
		}
		return nil
	}
	// Sin sesión: pedir canal de QR antes de conectar.
	qrChan, err := c.wa.GetQRChannel(context.Background())
	if err != nil {
		return err
	}
	if err := c.wa.Connect(); err != nil {
		return err
	}
	go func() {
		for item := range qrChan {
			c.mu.Lock()
			if item.Event == whatsmeow.QRChannelEventCode {
				c.ultimoQR = item.Code
			} else {
				c.ultimoQR = "" // success / timeout / error
			}
			c.mu.Unlock()
		}
	}()
	return nil
}

// EnviarTexto envía un mensaje de texto a un número (formato local o internacional).
func (c *Cliente) EnviarTexto(numero, texto string) error {
	if c.wa.Store.ID == nil {
		return errors.New("WhatsApp no está vinculado todavía")
	}
	jid := types.NewJID(normalizarNumero(numero), types.DefaultUserServer)
	_, err := c.wa.SendMessage(context.Background(), jid, &waE2E.Message{
		Conversation: proto.String(texto),
	})
	return err
}

// normalizarNumero deja solo dígitos y antepone el código de Perú (51) a los celulares locales.
func normalizarNumero(numero string) string {
	var b strings.Builder
	for _, r := range numero {
		if r >= '0' && r <= '9' {
			b.WriteRune(r)
		}
	}
	n := b.String()
	if len(n) == 9 && strings.HasPrefix(n, "9") {
		return "51" + n
	}
	return n
}
