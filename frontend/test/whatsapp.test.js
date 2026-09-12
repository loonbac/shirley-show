import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const modulePath = resolve(process.cwd(), 'frontend/js/modules/events/whatsapp.js');
const {
  cleanPhoneNumber,
  buildTicketPublicUrl,
  buildWhatsAppMessage,
  buildWhatsAppTicketLink,
} = await import(pathToFileURL(modulePath).href);

console.log('Running WhatsApp unit tests...');

// 1. Phone number cleaning and Peruvian prefix 51
{
  assert.equal(cleanPhoneNumber('987654321'), '51987654321', '9 digits must prepend 51');
  assert.equal(cleanPhoneNumber('+51 987 654 321'), '51987654321', 'Stripping non-digits with +51 prefix keeps 51987654321');
  assert.equal(cleanPhoneNumber('(+51) 987-654-321'), '51987654321', 'Stripping parentheses, plus, dashes, spaces');
  assert.equal(cleanPhoneNumber('(987) 654-321'), '51987654321', '9 digits in parentheses prepends 51');
  assert.equal(cleanPhoneNumber('  912345678  '), '51912345678', 'Trimming spaces on 9 digits');
  assert.equal(cleanPhoneNumber('51987654321'), '51987654321', 'Already 11 digits starting with 51 remains untouched');
  assert.equal(cleanPhoneNumber(''), '', 'Empty phone returns empty string');
}

// 2. Public ticket URL building
{
  const url1 = buildTicketPublicUrl('tok_abc123', 'https://shirleyshow.pe');
  assert.equal(url1, 'https://shirleyshow.pe/#/ticket/tok_abc123');

  const urlTrailingSlash = buildTicketPublicUrl('tok_xyz987', 'https://shirleyshow.pe/');
  assert.equal(urlTrailingSlash, 'https://shirleyshow.pe/#/ticket/tok_xyz987');

  const urlNoOrigin = buildTicketPublicUrl('tok_local');
  assert.equal(urlNoOrigin, '/#/ticket/tok_local');
}

// 3. WhatsApp message for Paid ticket
{
  const msg = buildWhatsAppMessage({
    buyerName: 'Carlos Mendoza',
    eventName: 'Noche Criolla Barranco',
    ticketUrl: 'https://shirleyshow.pe/#/ticket/tok_carlos',
    status: 'Pagado',
    price: 35.0,
  });

  assert.ok(msg.includes('Carlos Mendoza'), 'Message includes buyer name');
  assert.ok(msg.includes('Noche Criolla Barranco'), 'Message includes event name');
  assert.ok(msg.includes('https://shirleyshow.pe/#/ticket/tok_carlos'), 'Message includes public ticket URL');
  assert.ok(!msg.includes('pendiente') && !msg.includes('Pendiente'), 'Paid message must not say pending');
}

// 4. WhatsApp message for Pending reservation
{
  const msg = buildWhatsAppMessage({
    buyerName: 'Lucía Vargas',
    eventName: 'Cena Romántica',
    ticketUrl: 'https://shirleyshow.pe/#/ticket/tok_lucia',
    status: 'Pendiente',
    price: 50.0,
  });

  assert.ok(msg.includes('Lucía Vargas'), 'Message includes buyer name');
  assert.ok(msg.includes('Cena Romántica'), 'Message includes event name');
  assert.ok(msg.includes('50.00'), 'Message includes pending price');
  assert.ok(msg.toLowerCase().includes('pendiente'), 'Pending message informs pending status');
  assert.ok(msg.toLowerCase().includes('yape') || msg.toLowerCase().includes('pago'), 'Pending message mentions payment instructions');
  assert.ok(msg.includes('https://shirleyshow.pe/#/ticket/tok_lucia'), 'Message includes ticket reservation link');
}

// 5. Full wa.me link construction with secure encoding
{
  const linkPaid = buildWhatsAppTicketLink({
    ticket: {
      contacto: '+51 987 654 321',
      nombre_comprador: 'Carlos Mendoza',
      scan_token: 'tok_carlos',
      estado_pago: 'Pagado',
      precio: 35.0,
    },
    eventName: 'Noche Criolla Barranco',
    origin: 'https://shirleyshow.pe',
  });

  assert.ok(linkPaid.startsWith('https://wa.me/51987654321?text='), 'Link starts with clean wa.me phone target');
  const urlObj = new URL(linkPaid);
  const textParam = urlObj.searchParams.get('text');
  assert.ok(textParam.includes('Carlos Mendoza'), 'Decoded text parameter contains buyer name');
  assert.ok(textParam.includes('https://shirleyshow.pe/#/ticket/tok_carlos'), 'Decoded text contains ticket URL');

  const linkPending = buildWhatsAppTicketLink({
    ticket: {
      contacto: '912345678',
      nombre_comprador: 'Lucía Vargas',
      scan_token: 'tok_lucia',
      estado_pago: 'Pendiente',
      precio: 50.0,
    },
    eventName: 'Cena Romántica',
    origin: 'https://shirleyshow.pe',
  });

  assert.ok(linkPending.startsWith('https://wa.me/51912345678?text='), 'Link targets cleaned phone with 51 prepended');
  const urlObjPending = new URL(linkPending);
  const textParamPending = urlObjPending.searchParams.get('text');
  assert.ok(textParamPending.includes('50.00'), 'Decoded text contains pending amount');
  assert.ok(textParamPending.includes('https://shirleyshow.pe/#/ticket/tok_lucia'), 'Decoded text contains ticket URL');
}

console.log('WhatsApp unit tests passed successfully.');
