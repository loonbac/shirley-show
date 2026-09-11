use serde::{Deserialize, Serialize};

use crate::domain::ticketing::ticket::Ticket;
use crate::domain::ticketing::{
    Event, EventSummary, Expense, ExpenseCategoryBreakdown, Money, PaymentMethodBreakdown,
    TicketType,
};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketTypeDto {
    pub nombre: String,
    pub precio: f64,
    pub capacidad: u32,
}

impl TicketTypeDto {
    pub fn to_domain(&self) -> Result<TicketType, AppError> {
        let precio = Money::from_soles(self.precio)?;
        TicketType::new(self.nombre.clone(), precio, self.capacidad)
    }

    pub fn from_domain(t: &TicketType) -> Self {
        Self {
            nombre: t.nombre.clone(),
            precio: t.precio.to_soles(),
            capacidad: t.capacidad,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDto {
    pub id: String,
    pub nombre: String,
    pub fecha_hora: String,
    pub lugar: String,
    pub tipos_de_entrada: Vec<TicketTypeDto>,
    pub finalizado: bool,
    pub imagen: Option<String>,
    pub fecha_creacion: String,
    pub fecha_actualizacion: String,
}

impl EventDto {
    pub fn from_domain(e: &Event) -> Self {
        Self {
            id: e.id.clone(),
            nombre: e.nombre.clone(),
            fecha_hora: e.fecha_hora.clone(),
            lugar: e.lugar.clone(),
            tipos_de_entrada: e.tipos_de_entrada.iter().map(TicketTypeDto::from_domain).collect(),
            finalizado: e.finalizado,
            imagen: e.imagen.clone(),
            fecha_creacion: e.fecha_creacion.clone(),
            fecha_actualizacion: e.fecha_actualizacion.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEventRequest {
    pub nombre: String,
    pub fecha_hora: String,
    pub lugar: String,
    pub tipos_de_entrada: Vec<TicketTypeDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateEventRequest {
    pub nombre: String,
    pub fecha_hora: String,
    pub lugar: String,
    pub tipos_de_entrada: Vec<TicketTypeDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventResponse {
    pub evento: EventDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListEventsResponse {
    pub eventos: Vec<EventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseDto {
    pub id: String,
    pub evento_id: String,
    pub categoria: String,
    pub monto: f64,
    pub descripcion: String,
    pub fecha: String,
    pub fecha_creacion: String,
    pub fecha_actualizacion: String,
}

impl ExpenseDto {
    pub fn from_domain(g: &Expense) -> Self {
        Self {
            id: g.id.clone(),
            evento_id: g.evento_id.clone(),
            categoria: g.categoria.as_str().to_string(),
            monto: g.monto.to_soles(),
            descripcion: g.descripcion.clone(),
            fecha: g.fecha.clone(),
            fecha_creacion: g.fecha_creacion.clone(),
            fecha_actualizacion: g.fecha_actualizacion.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateExpenseRequest {
    pub categoria: String,
    pub monto: f64,
    pub descripcion: String,
    pub fecha: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateExpenseRequest {
    pub categoria: String,
    pub monto: f64,
    pub descripcion: String,
    pub fecha: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpenseResponse {
    pub gasto: ExpenseDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListExpensesResponse {
    pub gastos: Vec<ExpenseDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodBreakdownDto {
    pub yape_cobrado: f64,
    pub yape_pendiente: f64,
    pub yape_entradas: u32,
    pub efectivo_cobrado: f64,
    pub efectivo_pendiente: f64,
    pub efectivo_entradas: u32,
    pub cortesia_entradas: u32,
}

impl PaymentMethodBreakdownDto {
    pub fn from_domain(m: &PaymentMethodBreakdown) -> Self {
        Self {
            yape_cobrado: m.yape_cobrado.to_soles(),
            yape_pendiente: m.yape_pendiente.to_soles(),
            yape_entradas: m.yape_entradas,
            efectivo_cobrado: m.efectivo_cobrado.to_soles(),
            efectivo_pendiente: m.efectivo_pendiente.to_soles(),
            efectivo_entradas: m.efectivo_entradas,
            cortesia_entradas: m.cortesia_entradas,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseCategoryBreakdownDto {
    pub local_alquiler: f64,
    pub sonido_luces: f64,
    pub musicos_personal: f64,
    pub publicidad_marketing: f64,
    pub bebidas_catering: f64,
    pub transporte: f64,
    pub otros: f64,
}

impl ExpenseCategoryBreakdownDto {
    pub fn from_domain(c: &ExpenseCategoryBreakdown) -> Self {
        Self {
            local_alquiler: c.local_alquiler.to_soles(),
            sonido_luces: c.sonido_luces.to_soles(),
            musicos_personal: c.musicos_personal.to_soles(),
            publicidad_marketing: c.publicidad_marketing.to_soles(),
            bebidas_catering: c.bebidas_catering.to_soles(),
            transporte: c.transporte.to_soles(),
            otros: c.otros.to_soles(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummaryDto {
    pub cobrado: f64,
    pub pendiente: f64,
    pub total_proyectado: f64,
    pub gastos: f64,
    pub ganancia_neta: f64,
    pub capacidad_total: u32,
    pub entradas_emitidas: u32,
    pub aforo_proyectado: u32,
    pub desglose_metodos_pago: PaymentMethodBreakdownDto,
    pub desglose_gastos: ExpenseCategoryBreakdownDto,
}

impl EventSummaryDto {
    pub fn from_domain(s: &EventSummary) -> Self {
        Self {
            cobrado: s.cobrado.to_soles(),
            pendiente: s.pendiente.to_soles(),
            total_proyectado: s.total_proyectado.to_soles(),
            gastos: s.gastos.to_soles(),
            ganancia_neta: s.ganancia_neta_soles,
            capacidad_total: s.capacidad_total,
            entradas_emitidas: s.entradas_emitidas,
            aforo_proyectado: s.aforo_proyectado,
            desglose_metodos_pago: PaymentMethodBreakdownDto::from_domain(&s.metodos_pago),
            desglose_gastos: ExpenseCategoryBreakdownDto::from_domain(&s.categorias_gasto),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EventSummaryResponse {
    pub resumen: EventSummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDto {
    pub id: String,
    pub evento_id: String,
    pub interesado_id: Option<String>,
    pub nombre_comprador: String,
    pub contacto: String,
    pub tipo_entrada: String,
    pub precio: f64,
    pub medio_pago: String,
    pub estado_pago: String,
    pub scan_token: String,
    pub ticket_path: String,
    pub estado_checkin: String,
    pub hora_ingreso: Option<String>,
    pub qr_svg: Option<String>,
    pub fecha_creacion: String,
    pub fecha_actualizacion: String,
}

impl TicketDto {
    pub fn from_domain(t: &Ticket, include_qr: bool) -> Self {
        let qr_svg = if include_qr {
            t.render_qr_svg()
        } else {
            None
        };
        let ticket_path = format!("#/ticket/{}", t.scan_token.as_str());

        Self {
            id: t.id.clone(),
            evento_id: t.evento_id.clone(),
            interesado_id: t.interesado_id.clone(),
            nombre_comprador: t.nombre_comprador.clone(),
            contacto: t.contacto.clone(),
            tipo_entrada: t.tipo_entrada.clone(),
            precio: t.precio.to_soles(),
            medio_pago: t.medio_pago.as_str().to_string(),
            estado_pago: t.estado_pago.as_str().to_string(),
            scan_token: t.scan_token.as_str().to_string(),
            ticket_path,
            estado_checkin: t.estado_checkin.as_str().to_string(),
            hora_ingreso: t.hora_ingreso.clone(),
            qr_svg,
            fecha_creacion: t.fecha_creacion.clone(),
            fecha_actualizacion: t.fecha_actualizacion.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SellTicketsRequest {
    pub nombre_comprador: String,
    pub contacto: String,
    pub tipo_entrada: String,
    pub cantidad: u32,
    pub medio_pago: String,
    pub estado_pago: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SellTicketsResponse {
    pub entradas: Vec<TicketDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListTicketsResponse {
    pub entradas: Vec<TicketDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TicketResponse {
    pub entrada: TicketDto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTicketContactRequest {
    pub nombre_comprador: String,
    pub contacto: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckInRequest {
    pub evento_id: String,
    pub codigo: String,
}
