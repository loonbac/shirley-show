/**
 * WhatsApp link generation and phone sanitation for Shirley Show ticketing.
 * Pure transformation functions without side effects or DOM dependencies.
 */

/**
 * Strips non-digit characters from a phone number string and prepends Peruvian prefix 51
 * if the number contains exactly 9 digits without country code.
 *
 * @param {string | number} phone 
 * @returns {string} Clean digits string suitable for wa.me URL.
 */
export function cleanPhoneNumber(phone) {
  if (!phone) return '';
  const digits = String(phone).replace(/\D/g, '');
  if (digits.length === 9) {
    return `51${digits}`;
  }
  return digits;
}

/**
 * Builds the absolute or relative public ticket URL for a given scan_token.
 *
 * @param {string} scanToken 
 * @param {string} [origin=''] Optional base URL origin (e.g., window.location.origin)
 * @returns {string} Public ticket hash URL
 */
export function buildTicketPublicUrl(scanToken, origin = '') {
  const base = origin ? String(origin).replace(/\/+$/, '') : '';
  const token = scanToken ? encodeURIComponent(String(scanToken).trim()) : '';
  return `${base}/#/ticket/${token}`;
}

/**
 * Generates the user-facing text message to be shared via WhatsApp.
 *
 * @param {{
 *   buyerName: string,
 *   eventName: string,
 *   ticketUrl: string,
 *   status: string,
 *   price?: number,
 *   paymentMedium?: string,
 * }} params 
 * @returns {string} Plaintext message
 */
export function buildWhatsAppMessage(params) {
  const {
    buyerName = '',
    eventName = '',
    ticketUrl = '',
    status = 'Pagado',
    price = 0,
  } = params;

  const name = buyerName.trim();
  const event = eventName.trim();
  const isPaid = status === 'Pagado';

  if (isPaid) {
    return (
      `¡Hola ${name}! Aquí tienes tu entrada para ${event}:\n` +
      `${ticketUrl}\n\n` +
      `Guarda este enlace: ahí puedes ver tu entrada cuando la necesites. ` +
      `En la puerta basta con dar tu nombre. ¡Te esperamos!`
    );
  }

  // Status is Pendiente
  const formattedPrice = Number(price || 0).toFixed(2);
  return (
    `¡Hola ${name}! Tienes una reserva pendiente para ${event}.\n` +
    `Monto a pagar: S/ ${formattedPrice}\n\n` +
    `Por favor coordina tu pago por Yape o Efectivo para habilitar tu código QR de ingreso.\n` +
    `Enlace de tu reserva:\n` +
    `${ticketUrl}`
  );
}

/**
 * Constructs the full wa.me direct URL with securely encoded message text.
 *
 * @param {{
 *   ticket: {
 *     contacto?: string,
 *     nombre_comprador?: string,
 *     scan_token?: string,
 *     estado_pago?: string,
 *     precio?: number,
 *     medio_pago?: string,
 *   },
 *   eventName?: string,
 *   origin?: string,
 * }} options 
 * @returns {string} https://wa.me/... URL
 */
export function buildWhatsAppTicketLink(options) {
  const { ticket = {}, eventName = '', origin } = options;

  const defaultOrigin =
    origin !== undefined
      ? origin
      : typeof window !== 'undefined' && window.location && window.location.origin
      ? window.location.origin
      : '';

  const cleanPhone = cleanPhoneNumber(ticket.contacto || '');
  const ticketUrl = buildTicketPublicUrl(ticket.scan_token || '', defaultOrigin);
  const message = buildWhatsAppMessage({
    buyerName: ticket.nombre_comprador || '',
    eventName: eventName || '',
    ticketUrl,
    status: ticket.estado_pago || 'Pagado',
    price: ticket.precio || 0,
    paymentMedium: ticket.medio_pago || 'Yape',
  });

  const encodedText = encodeURIComponent(message);
  return `https://wa.me/${cleanPhone}?text=${encodedText}`;
}
