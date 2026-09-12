import { requestJson } from '../../data/request-json.js';

/**
 * Ticketing HTTP API Client.
 * Cohesive client encapsulating all backend endpoints for events, summary,
 * tickets, leads (interesados), and expenses (gastos).
 */

// --- Events ---

/**
 * Lists all events. By default includes active and finalized events if requested.
 * @param {{ incluir_finalizados?: boolean }} [options={}]
 * @returns {Promise<Array<any>>}
 */
export async function listEvents(options = {}) {
  const query = options.incluir_finalizados ? '?incluir_finalizados=true' : '';
  return requestJson(`/api/eventos${query}`, { method: 'GET' });
}

/**
 * Retrieves details for a specific event by ID.
 * @param {string} id 
 * @returns {Promise<any>}
 */
export async function getEvent(id) {
  return requestJson(`/api/eventos/${encodeURIComponent(id)}`, { method: 'GET' });
}

/**
 * Creates a new event.
 * @param {{
 *   nombre: string,
 *   lugar: string,
 *   fecha_hora: string,
 *   tipos_de_entrada: Array<{ nombre: string, precio: number, capacidad: number }>
 * }} data 
 * @returns {Promise<any>}
 */
export async function createEvent(data) {
  return requestJson('/api/eventos', {
    method: 'POST',
    body: data,
  });
}

/**
 * Updates metadata or ticket types of an event.
 * @param {string} id 
 * @param {{
 *   nombre?: string,
 *   lugar?: string,
 *   fecha_hora?: string,
 *   tipos_de_entrada?: Array<{ nombre: string, precio: number, capacidad: number }>
 * }} data 
 * @returns {Promise<any>}
 */
export async function updateEvent(id, data) {
  return requestJson(`/api/eventos/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: data,
  });
}

/**
 * Deletes an event and cascades to all child records and poster.
 * @param {string} id 
 * @returns {Promise<{ deleted: boolean, cleanup_pending: boolean }>}
 */
export async function deleteEvent(id) {
  return requestJson(`/api/eventos/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

/**
 * Marks an event as finalized.
 * @param {string} id 
 * @returns {Promise<any>}
 */
export async function finalizeEvent(id) {
  return requestJson(`/api/eventos/${encodeURIComponent(id)}/finalizar`, {
    method: 'POST',
  });
}

/**
 * Reopens a finalized event.
 * @param {string} id 
 * @returns {Promise<any>}
 */
export async function reopenEvent(id) {
  return requestJson(`/api/eventos/${encodeURIComponent(id)}/reabrir`, {
    method: 'POST',
  });
}

/**
 * Uploads/replaces promotional poster for an event.
 * @param {string} id 
 * @param {File | Blob} file 
 * @returns {Promise<{ imagen: string }>}
 */
export async function uploadEventPoster(id, file) {
  const formData = new FormData();
  formData.append('imagen', file);
  return requestJson(`/api/eventos/${encodeURIComponent(id)}/imagen`, {
    method: 'POST',
    body: formData,
  });
}

/**
 * Returns the public image URL for an event's poster.
 * @param {string} id 
 * @returns {string}
 */
export function getEventPosterUrl(id) {
  return `/api/eventos/${encodeURIComponent(id)}/imagen`;
}

// --- Summary ---

/**
 * Fetches consolidated financial and capacity summary for an event.
 * @param {string} id 
 * @returns {Promise<{
 *   cobrado: number,
 *   pendiente: number,
 *   total_proyectado: number,
 *   gastos: number,
 *   ganancia_neta: number,
 *   capacidad_total: number,
 *   entradas_emitidas: number,
 *   aforo_proyectado: number,
 *   por_medio_pago: Record<string, { cobrado: number, pendiente: number, cantidad: number }>,
 *   gastos_por_categoria: Record<string, number>,
 * }>}
 */
export async function getEventSummary(id) {
  return requestJson(`/api/eventos/${encodeURIComponent(id)}/resumen`, {
    method: 'GET',
  });
}

// --- Tickets (Entradas) ---

/**
 * Lists tickets for an event with optional filters.
 * @param {string} eventId 
 * @param {{ q?: string, estado_pago?: string, tipo_entrada?: string }} [filters={}]
 * @returns {Promise<Array<any>>}
 */
export async function listTickets(eventId, filters = {}) {
  const params = new URLSearchParams();
  if (filters.q) params.set('q', filters.q);
  if (filters.estado_pago) params.set('estado_pago', filters.estado_pago);
  if (filters.tipo_entrada) params.set('tipo_entrada', filters.tipo_entrada);

  const query = params.toString() ? `?${params.toString()}` : '';
  return requestJson(`/api/eventos/${encodeURIComponent(eventId)}/entradas${query}`, {
    method: 'GET',
  });
}

/**
 * Sells N tickets in a single atomic transaction.
 * @param {string} eventId 
 * @param {{
 *   nombre_comprador: string,
 *   contacto: string,
 *   tipo_entrada: string,
 *   cantidad: number,
 *   medio_pago: 'Yape' | 'Efectivo' | 'Cortesía',
 *   estado_pago: 'Pagado' | 'Pendiente',
 * }} data 
 * @returns {Promise<Array<any>>}
 */
export async function sellTickets(eventId, data) {
  return requestJson(`/api/eventos/${encodeURIComponent(eventId)}/entradas`, {
    method: 'POST',
    body: data,
  });
}

/**
 * Retrieves a single ticket by ID.
 * @param {string} ticketId 
 * @returns {Promise<any>}
 */
export async function getTicket(ticketId) {
  return requestJson(`/api/entradas/${encodeURIComponent(ticketId)}`, {
    method: 'GET',
  });
}

/**
 * Updates contact information for an issued ticket.
 * @param {string} ticketId 
 * @param {{ nombre_comprador?: string, contacto?: string }} data 
 * @returns {Promise<any>}
 */
export async function updateTicketContact(ticketId, data) {
  return requestJson(`/api/entradas/${encodeURIComponent(ticketId)}`, {
    method: 'PATCH',
    body: data,
  });
}

/**
 * Deletes a ticket if it has not checked in.
 * @param {string} ticketId 
 * @returns {Promise<void>}
 */
export async function deleteTicket(ticketId) {
  return requestJson(`/api/entradas/${encodeURIComponent(ticketId)}`, {
    method: 'DELETE',
  });
}

/**
 * Confirms payment for a pending ticket.
 * @param {string} ticketId 
 * @returns {Promise<any>}
 */
export async function confirmTicketPayment(ticketId) {
  return requestJson(`/api/entradas/${encodeURIComponent(ticketId)}/confirmar-pago`, {
    method: 'POST',
  });
}

/**
 * Validates check-in at the door.
 * @param {string} eventId 
 * @param {string} codigo 
 * @returns {Promise<{
 *   resultado: 'Valida' | 'Ya Usada' | 'Invalida',
 *   comprador?: string,
 *   tipo_entrada?: string,
 *   hora_ingreso?: string,
 *   motivo?: string,
 * }>}
 */
export async function checkInTicket(eventId, codigo) {
  return requestJson('/api/entradas/checkin', {
    method: 'POST',
    body: { evento_id: eventId, codigo },
  });
}

/**
 * Fetches public ticket representation by scan_token.
 * @param {string} scanToken 
 * @returns {Promise<{
 *   nombre_evento: string,
 *   fecha_hora: string,
 *   lugar: string,
 *   nombre_comprador: string,
 *   tipo_entrada: string,
 *   precio: number,
 *   estado_pago: 'Pagado' | 'Pendiente',
 *   estado_checkin: 'No ingresó' | 'Ingresó',
 *   qr_svg: string | null,
 * }>}
 */
export async function getPublicTicket(scanToken) {
  return requestJson(`/api/public/ticket/${encodeURIComponent(scanToken)}`, {
    method: 'GET',
  });
}

// --- Interests (Interesados) ---

/**
 * Lists interested leads for an event.
 * @param {string} eventId 
 * @returns {Promise<Array<any>>}
 */
export async function listInterests(eventId) {
  return requestJson(`/api/eventos/${encodeURIComponent(eventId)}/interesados`, {
    method: 'GET',
  });
}

/**
 * Registers an interested lead.
 * @param {string} eventId 
 * @param {{
 *   nombre: string,
 *   contacto: string,
 *   canal: string,
 *   promotora: string,
 *   personas: number,
 *   notas?: string,
 * }} data 
 * @returns {Promise<any>}
 */
export async function createInterest(eventId, data) {
  return requestJson(`/api/eventos/${encodeURIComponent(eventId)}/interesados`, {
    method: 'POST',
    body: data,
  });
}

/**
 * Updates an interested lead record.
 * @param {string} interestId 
 * @param {{
 *   nombre?: string,
 *   contacto?: string,
 *   canal?: string,
 *   promotora?: string,
 *   personas?: number,
 *   estado?: 'Pendiente' | 'Confirmado' | 'Cancelado',
 *   notas?: string,
 * }} data 
 * @returns {Promise<any>}
 */
export async function updateInterest(interestId, data) {
  return requestJson(`/api/interesados/${encodeURIComponent(interestId)}`, {
    method: 'PATCH',
    body: data,
  });
}

/**
 * Deletes an interested lead record.
 * @param {string} interestId 
 * @returns {Promise<void>}
 */
export async function deleteInterest(interestId) {
  return requestJson(`/api/interesados/${encodeURIComponent(interestId)}`, {
    method: 'DELETE',
  });
}

/**
 * Atomically converts an interested lead into ticket sales.
 * @param {string} interestId 
 * @param {{
 *   tipo_entrada: string,
 *   cantidad: number,
 *   medio_pago: 'Yape' | 'Efectivo' | 'Cortesía',
 *   estado_pago: 'Pagado' | 'Pendiente',
 * }} data 
 * @returns {Promise<{ interesado: any, entradas: Array<any> }>}
 */
export async function convertInterestToSale(interestId, data) {
  return requestJson(`/api/interesados/${encodeURIComponent(interestId)}/vender`, {
    method: 'POST',
    body: data,
  });
}

// --- Expenses (Gastos) ---

/**
 * Lists expenses for an event.
 * @param {string} eventId 
 * @returns {Promise<Array<any>>}
 */
export async function listExpenses(eventId) {
  return requestJson(`/api/eventos/${encodeURIComponent(eventId)}/gastos`, {
    method: 'GET',
  });
}

/**
 * Records a new expense for an event.
 * @param {string} eventId 
 * @param {{
 *   categoria: string,
 *   monto: number,
 *   descripcion: string,
 *   fecha: string,
 * }} data 
 * @returns {Promise<any>}
 */
export async function createExpense(eventId, data) {
  return requestJson(`/api/eventos/${encodeURIComponent(eventId)}/gastos`, {
    method: 'POST',
    body: data,
  });
}

/**
 * Updates an existing expense.
 * @param {string} expenseId 
 * @param {{
 *   categoria?: string,
 *   monto?: number,
 *   descripcion?: string,
 *   fecha?: string,
 * }} data 
 * @returns {Promise<any>}
 */
export async function updateExpense(expenseId, data) {
  return requestJson(`/api/gastos/${encodeURIComponent(expenseId)}`, {
    method: 'PATCH',
    body: data,
  });
}

/**
 * Deletes an expense.
 * @param {string} expenseId 
 * @returns {Promise<void>}
 */
export async function deleteExpense(expenseId) {
  return requestJson(`/api/gastos/${encodeURIComponent(expenseId)}`, {
    method: 'DELETE',
  });
}
