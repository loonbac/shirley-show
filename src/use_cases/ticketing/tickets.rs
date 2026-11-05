use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::ticketing::ticket::{PaymentStatus, Ticket};
use crate::error::AppError;
use crate::ports::ticketing::{
    CredentialGenerator, EventRepository, SellTicketsInput, TicketRepository, TicketingTransactions,
};

#[derive(Clone)]
pub struct SellTickets<E, T, C> {
    pub _event_repo: Arc<E>,
    pub tx_repo: Arc<T>,
    pub credentials_gen: Arc<C>,
}

impl<E: EventRepository, T: TicketingTransactions, C: CredentialGenerator> SellTickets<E, T, C> {
    pub fn new(event_repo: Arc<E>, tx_repo: Arc<T>, credentials_gen: Arc<C>) -> Self {
        Self {
            _event_repo: event_repo,
            tx_repo,
            credentials_gen,
        }
    }

    pub fn execute(&self, input: SellTicketsInput) -> Result<Vec<Ticket>, AppError> {
        if input.cantidad < 1 {
            return Err(AppError::bad_request(
                "invalid_quantity",
                "La cantidad de entradas a vender debe ser de al menos 1",
            ));
        }

        let mut creds = Vec::with_capacity(input.cantidad as usize);
        for _ in 0..input.cantidad {
            creds.push(self.credentials_gen.generate()?);
        }

        self.tx_repo.issue_sale(&input, creds)
    }
}

#[derive(Clone)]
pub struct ConfirmTicketPayment<R> {
    ticket_repo: Arc<R>,
}

impl<R: TicketRepository> ConfirmTicketPayment<R> {
    pub fn new(ticket_repo: Arc<R>) -> Self {
        Self { ticket_repo }
    }

    pub fn execute(&self, id: &str) -> Result<Ticket, AppError> {
        match self.ticket_repo.confirm_ticket_payment(id)? {
            Some(ticket) => Ok(ticket),
            None => Err(AppError::not_found(
                "ticket_not_found",
                "Entrada no encontrada",
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicTicketDto {
    pub nombre_evento: String,
    pub fecha_hora: String,
    pub lugar: String,
    pub nombre_comprador: String,
    pub tipo_entrada: String,
    pub precio: f64,
    pub estado_pago: String,
    pub estado_checkin: String,
    /// The buyer's entry credential, as an SVG.
    ///
    /// It stays: without it the buyer cannot enter the venue, and this route is
    /// how the WhatsApp link shows the ticket. The `scan_token` in that link is
    /// the bearer credential by design (64 hex, unguessable).
    pub qr_svg: Option<String>,
    // No raw `codigo_qr`, on purpose.
    //
    // The SVG already encodes it for the door to scan, so the hex string adds no
    // capability to the buyer; it only makes the secret trivially copyable out of
    // the JSON. Anything holding the SVG could decode it anyway, so this is a
    // reduction in exposure, not a boundary.
}

#[derive(Clone)]
pub struct GetPublicTicket<E, T> {
    event_repo: Arc<E>,
    ticket_repo: Arc<T>,
    /// Base URL new tickets wrap their door secret in.
    ///
    /// `None` keeps the bare 32-hex payload, which is what every ticket printed
    /// before this change carries.
    public_base_url: Option<String>,
}

impl<E: EventRepository, T: TicketRepository> GetPublicTicket<E, T> {
    pub fn new(event_repo: Arc<E>, ticket_repo: Arc<T>) -> Self {
        Self {
            event_repo,
            ticket_repo,
            public_base_url: None,
        }
    }

    pub fn with_public_base_url(mut self, url: Option<String>) -> Self {
        self.public_base_url = url;
        self
    }

    /// Resolves the ticket behind either credential the buyer can arrive with.
    ///
    /// The buyer is never sent a link: what they receive is the photo of their
    /// ticket, and the QR inside it carries `<base>/#/ticket/<codigo_qr>`. So the
    /// 32-hex door secret is the *only* thing that reaches this route from a
    /// phone camera, and refusing it here left the buyer staring at a landing
    /// page with no ticket at all -- the one document they own, invisible.
    ///
    /// `scan_token` stays first because it is the narrower credential: it is
    /// minted per ticket for the buyer's link and is not what the door compares.
    /// Falling back to `codigo_qr` does not widen what a stranger can read: the
    /// QR the buyer already holds is the door credential, and this route answers
    /// with the same ticket either way.
    pub fn execute(&self, token: &str) -> Result<PublicTicketDto, AppError> {
        let ticket = match self.ticket_repo.get_ticket_by_scan_token(token)? {
            Some(t) => t,
            None => match self.ticket_repo.get_ticket_by_qr_secret(token)? {
                Some(t) => t,
                None => {
                    return Err(AppError::not_found(
                        "ticket_not_found",
                        "Entrada no encontrada",
                    ));
                }
            },
        };

        let event = match self.event_repo.get_event(&ticket.evento_id)? {
            Some(e) => e,
            None => {
                return Err(AppError::not_found(
                    "event_not_found",
                    "Evento no encontrado",
                ));
            }
        };

        let qr_svg = ticket.render_qr_svg(self.public_base_url.as_deref());

        Ok(PublicTicketDto {
            nombre_evento: event.nombre,
            fecha_hora: event.fecha_hora,
            lugar: event.lugar,
            nombre_comprador: ticket.nombre_comprador,
            tipo_entrada: ticket.tipo_entrada,
            precio: ticket.precio.to_soles(),
            estado_pago: ticket.estado_pago.as_str().to_string(),
            estado_checkin: ticket.estado_checkin.as_str().to_string(),
            // `render_qr_svg` already returns `None` for an unpaid ticket, so the
            // pending case cannot leak a code by omission here.
            qr_svg,
        })
    }
}

#[derive(Clone)]
pub struct ListTickets<R> {
    ticket_repo: Arc<R>,
}

impl<R: TicketRepository> ListTickets<R> {
    pub fn new(ticket_repo: Arc<R>) -> Self {
        Self { ticket_repo }
    }

    pub fn execute(
        &self,
        evento_id: &str,
        q: Option<&str>,
        estado_pago: Option<&str>,
        tipo_entrada: Option<&str>,
    ) -> Result<Vec<Ticket>, AppError> {
        self.ticket_repo
            .list_tickets(evento_id, q, estado_pago, tipo_entrada)
    }
}

#[derive(Clone)]
pub struct GetTicket<R> {
    ticket_repo: Arc<R>,
}

impl<R: TicketRepository> GetTicket<R> {
    pub fn new(ticket_repo: Arc<R>) -> Self {
        Self { ticket_repo }
    }

    pub fn execute(&self, id: &str) -> Result<Ticket, AppError> {
        match self.ticket_repo.get_ticket(id)? {
            Some(t) => Ok(t),
            None => Err(AppError::not_found(
                "ticket_not_found",
                "Entrada no encontrada",
            )),
        }
    }
}

#[derive(Clone)]
pub struct UpdateTicketContact<R> {
    ticket_repo: Arc<R>,
}

impl<R: TicketRepository> UpdateTicketContact<R> {
    pub fn new(ticket_repo: Arc<R>) -> Self {
        Self { ticket_repo }
    }

    pub fn execute(&self, id: &str, nombre: &str, contacto: &str) -> Result<Ticket, AppError> {
        match self
            .ticket_repo
            .update_ticket_contact(id, nombre, contacto)?
        {
            Some(t) => Ok(t),
            None => Err(AppError::not_found(
                "ticket_not_found",
                "Entrada no encontrada",
            )),
        }
    }
}

#[derive(Clone)]
pub struct DeleteTicket<R> {
    ticket_repo: Arc<R>,
}

impl<R: TicketRepository> DeleteTicket<R> {
    pub fn new(ticket_repo: Arc<R>) -> Self {
        Self { ticket_repo }
    }

    pub fn execute(&self, id: &str) -> Result<(), AppError> {
        let deleted = self.ticket_repo.delete_ticket(id)?;
        if !deleted {
            return Err(AppError::not_found(
                "ticket_not_found",
                "Entrada no encontrada",
            ));
        }
        Ok(())
    }
}
