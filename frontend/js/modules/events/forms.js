import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../../components/atoms/button.js';
import { createTextInput } from '../../components/atoms/text-input.js';
import { bindDialogDismiss } from '../../behaviors/bind-dialog-dismiss.js';
import { bindFormSubmit } from '../../behaviors/bind-form-submit.js';
import { trimRequired } from '../../utils/validation/trim-required.js';
import { buildTicketPublicUrl } from './whatsapp.js';
import { generarFotoEntrada, descargarBlob } from './ticket-photo.js';

/**
 * Creates an accessible modal dialog for creating or editing an event.
 * @param {{
 *   event?: any,
 *   onSave: (data: {
 *     nombre: string,
 *     lugar: string,
 *     fecha_hora: string,
 *     tipos_de_entrada: Array<{ nombre: string, precio: number, capacidad: number }>
 *   }) => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createEventFormDialog(props) {
  const { event = null, onSave, onCancel } = props;
  const isEdit = Boolean(event);
  const titleText = isEdit ? 'Editar Evento' : 'Nuevo Evento';

  const titleEl = createElement('h3', { class: 'events-dialog-title' }, [titleText]);

  const nombreInput = createTextInput({
    label: 'Nombre del Evento',
    name: 'nombre',
    value: (event && event.nombre) || '',
    placeholder: 'ej. Cena Show Criollo',
    required: true,
  });

  const lugarInput = createTextInput({
    label: 'Lugar / Local',
    name: 'lugar',
    value: (event && event.lugar) || '',
    placeholder: 'ej. La Estación de Barranco',
    required: true,
  });

  // Convert ISO string to datetime-local format (YYYY-MM-DDTHH:MM)
  let initialDateTime = '';
  if (event && event.fecha_hora) {
    try {
      const d = new Date(event.fecha_hora);
      const pad = (n) => String(n).padStart(2, '0');
      initialDateTime = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
    } catch {
      initialDateTime = '';
    }
  }

  const fechaInput = createTextInput({
    label: 'Fecha y Hora',
    name: 'fecha_hora',
    type: 'datetime-local',
    value: initialDateTime,
    required: true,
  });

  // Ticket types editor
  let ticketTypes = (event && Array.isArray(event.tipos_de_entrada) && event.tipos_de_entrada.length > 0)
    ? JSON.parse(JSON.stringify(event.tipos_de_entrada))
    : [{ nombre: 'General', precio: 30, capacidad: 50 }];

  const typesContainer = createElement('div', { class: 'events-types-list' });

  function renderTypes() {
    typesContainer.innerHTML = '';
    ticketTypes.forEach((t, idx) => {
      const row = createElement('div', { class: 'events-type-row' }, [
        createElement('input', {
          type: 'text',
          class: 'text-input events-type-name',
          placeholder: 'Tipo (ej. General)',
          value: t.nombre,
          required: 'true',
          onChange: (e) => { t.nombre = e.target.value.trim(); },
        }),
        createElement('input', {
          type: 'number',
          class: 'text-input events-type-price',
          placeholder: 'Precio S/',
          step: '0.5',
          min: '0',
          value: String(t.precio),
          required: 'true',
          onChange: (e) => { t.precio = parseFloat(e.target.value) || 0; },
        }),
        createElement('input', {
          type: 'number',
          class: 'text-input events-type-cap',
          placeholder: 'Capacidad',
          step: '1',
          min: '1',
          value: String(t.capacidad),
          required: 'true',
          onChange: (e) => { t.capacidad = parseInt(e.target.value, 10) || 1; },
        }),
        ticketTypes.length > 1
          ? createElement('button', {
              type: 'button',
              class: 'btn btn-ghost btn-sm events-type-del-btn',
              'aria-label': 'Eliminar tipo',
              onClick: () => {
                ticketTypes.splice(idx, 1);
                renderTypes();
              },
            }, ['✕'])
          : null,
      ].filter(Boolean));
      typesContainer.appendChild(row);
    });
  }
  renderTypes();

  const addTypeBtn = createButton({
    text: '+ Agregar Tipo de Entrada',
    variant: 'ghost',
    size: 'sm',
    type: 'button',
    onClick: () => {
      ticketTypes.push({ nombre: 'VIP', precio: 50, capacidad: 20 });
      renderTypes();
    },
  });

  const typesSection = createElement('div', { class: 'events-types-section' }, [
    createElement('label', { class: 'events-form-label' }, ['Tipos de Entrada:']),
    typesContainer,
    addTypeBtn.el,
  ]);

  const errorEl = createElement('div', { class: 'events-form-error', style: 'display: none;' });

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: isEdit ? 'Guardar Cambios' : 'Crear Evento',
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'events-dialog-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement('form', { class: 'events-form' }, [
    nombreInput.el,
    lugarInput.el,
    fechaInput.el,
    typesSection,
    errorEl,
    actionsEl,
  ]);

  const dialogCard = createElement(
    'div',
    { class: 'events-dialog', role: 'dialog', 'aria-modal': 'true' },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  const unbindDismiss = bindDialogDismiss(overlayEl, onCancel);
  const unbindSubmit = bindFormSubmit(formEl, async () => {
    errorEl.style.display = 'none';
    const nombre = nombreInput.el.querySelector('input').value.trim();
    const lugar = lugarInput.el.querySelector('input').value.trim();
    const fechaVal = fechaInput.el.querySelector('input').value;

    if (!nombre) {
      errorEl.textContent = 'El nombre del evento es requerido.';
      errorEl.style.display = 'block';
      return;
    }
    if (!lugar) {
      errorEl.textContent = 'El lugar del evento es requerido.';
      errorEl.style.display = 'block';
      return;
    }
    if (!fechaVal) {
      errorEl.textContent = 'La fecha y hora son requeridas.';
      errorEl.style.display = 'block';
      return;
    }
    if (ticketTypes.length === 0) {
      errorEl.textContent = 'Debe configurar al menos un tipo de entrada.';
      errorEl.style.display = 'block';
      return;
    }

    const isoDate = new Date(fechaVal).toISOString();

    try {
      await onSave({
        nombre,
        lugar,
        fecha_hora: isoDate,
        tipos_de_entrada: ticketTypes,
      });
    } catch (err) {
      errorEl.textContent = (err && err.message) || 'Error al guardar el evento.';
      errorEl.style.display = 'block';
    }
  });

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    if (overlayEl.parentNode) {
      overlayEl.parentNode.removeChild(overlayEl);
    }
  }

  return { el: overlayEl, destroy };
}

/**
 * Creates an accessible modal dialog for recording or editing an expense.
 * @param {{
 *   expense?: any,
 *   onSave: (data: { categoria: string, monto: number, descripcion: string, fecha: string }) => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createExpenseFormDialog(props) {
  const { expense = null, onSave, onCancel } = props;
  const isEdit = Boolean(expense);
  const titleText = isEdit ? 'Editar Gasto' : 'Registrar Gasto';

  const categories = [
    'Local/Alquiler',
    'Sonido/Luces',
    'Músicos/Personal',
    'Publicidad/Marketing',
    'Bebidas/Catering',
    'Transporte',
    'Otros',
  ];

  const titleEl = createElement('h3', { class: 'events-dialog-title' }, [titleText]);

  const catOptions = categories.map((cat) =>
    createElement('option', {
      value: cat,
      selected: expense && expense.categoria === cat ? 'selected' : undefined,
    }, [cat])
  );

  const selectCat = createElement('select', {
    class: 'text-input events-select',
    required: 'true',
  }, catOptions);

  const catField = createElement('div', { class: 'text-input-field' }, [
    createElement('label', { class: 'text-input-label' }, ['Categoría']),
    selectCat,
  ]);

  const montoInput = createTextInput({
    label: 'Monto (S/)',
    name: 'monto',
    type: 'number',
    step: '0.01',
    min: '0.01',
    value: expense ? String(expense.monto) : '',
    placeholder: '0.00',
    required: true,
  });

  const descInput = createTextInput({
    label: 'Descripción',
    name: 'descripcion',
    value: (expense && expense.descripcion) || '',
    placeholder: 'ej. Pago de alquiler de local',
    required: true,
  });

  const todayStr = new Date().toISOString().split('T')[0];
  const fechaInput = createTextInput({
    label: 'Fecha',
    name: 'fecha',
    type: 'date',
    value: (expense && expense.fecha) || todayStr,
    required: true,
  });

  const errorEl = createElement('div', { class: 'events-form-error', style: 'display: none;' });

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: isEdit ? 'Guardar Cambios' : 'Registrar Gasto',
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'events-dialog-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement('form', { class: 'events-form' }, [
    catField,
    montoInput.el,
    descInput.el,
    fechaInput.el,
    errorEl,
    actionsEl,
  ]);

  const dialogCard = createElement(
    'div',
    { class: 'events-dialog', role: 'dialog', 'aria-modal': 'true' },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  const unbindDismiss = bindDialogDismiss(overlayEl, onCancel);
  const unbindSubmit = bindFormSubmit(formEl, async () => {
    errorEl.style.display = 'none';
    const categoria = selectCat.value;
    const monto = parseFloat(montoInput.el.querySelector('input').value);
    const descripcion = descInput.el.querySelector('input').value.trim();
    const fecha = fechaInput.el.querySelector('input').value;

    if (!categoria || isNaN(monto) || monto <= 0 || !descripcion || !fecha) {
      errorEl.textContent = 'Todos los campos son requeridos y el monto debe ser mayor a 0.';
      errorEl.style.display = 'block';
      return;
    }

    try {
      await onSave({ categoria, monto, descripcion, fecha });
    } catch (err) {
      errorEl.textContent = (err && err.message) || 'Error al guardar el gasto.';
      errorEl.style.display = 'block';
    }
  });

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    if (overlayEl.parentNode) {
      overlayEl.parentNode.removeChild(overlayEl);
    }
  }

  return { el: overlayEl, destroy };
}

/**
 * Creates an accessible modal dialog for creating or editing an interested lead.
 * @param {{
 *   interest?: any,
 *   onSave: (data: any) => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createInterestFormDialog(props) {
  const { interest = null, onSave, onCancel } = props;
  const isEdit = Boolean(interest);
  const titleText = isEdit ? 'Editar Interesado' : 'Registrar Interesado';

  const titleEl = createElement('h3', { class: 'events-dialog-title' }, [titleText]);

  const nombreInput = createTextInput({
    label: 'Nombre de Contacto',
    name: 'nombre',
    value: (interest && interest.nombre) || '',
    placeholder: 'ej. Mariana Ríos',
    required: true,
  });

  const contactoInput = createTextInput({
    label: 'Teléfono / WhatsApp',
    name: 'contacto',
    value: (interest && interest.contacto) || '',
    placeholder: 'ej. 999888777',
    required: true,
  });

  const canalInput = createTextInput({
    label: 'Canal',
    name: 'canal',
    value: (interest && interest.canal) || 'WhatsApp',
    placeholder: 'WhatsApp, Instagram, etc.',
    required: true,
  });

  const promotoraInput = createTextInput({
    label: 'Promotora / Responsable',
    name: 'promotora',
    value: (interest && interest.promotora) || 'Shirley',
    placeholder: 'ej. Shirley',
    required: true,
  });

  const personasInput = createTextInput({
    label: 'Cantidad de Personas',
    name: 'personas',
    type: 'number',
    min: '1',
    value: interest ? String(interest.personas) : '1',
    required: true,
  });

  const notasInput = createTextInput({
    label: 'Notas adicionales (opcional)',
    name: 'notas',
    value: (interest && interest.notas) || '',
    placeholder: 'ej. Mesa preferencial',
  });

  let statusSelectField = null;
  let statusSelect = null;
  if (isEdit) {
    statusSelect = createElement('select', { class: 'text-input events-select' }, [
      createElement('option', { value: 'Pendiente', selected: interest.estado === 'Pendiente' ? 'selected' : undefined }, ['Pendiente']),
      createElement('option', { value: 'Confirmado', selected: interest.estado === 'Confirmado' ? 'selected' : undefined }, ['Confirmado']),
      createElement('option', { value: 'Cancelado', selected: interest.estado === 'Cancelado' ? 'selected' : undefined }, ['Cancelado']),
    ]);
    statusSelectField = createElement('div', { class: 'text-input-field' }, [
      createElement('label', { class: 'text-input-label' }, ['Estado']),
      statusSelect,
    ]);
  }

  const errorEl = createElement('div', { class: 'events-form-error', style: 'display: none;' });

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: isEdit ? 'Guardar Cambios' : 'Registrar Interesado',
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'events-dialog-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement('form', { class: 'events-form' }, [
    nombreInput.el,
    contactoInput.el,
    canalInput.el,
    promotoraInput.el,
    personasInput.el,
    statusSelectField,
    notasInput.el,
    errorEl,
    actionsEl,
  ].filter(Boolean));

  const dialogCard = createElement(
    'div',
    { class: 'events-dialog', role: 'dialog', 'aria-modal': 'true' },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  const unbindDismiss = bindDialogDismiss(overlayEl, onCancel);
  const unbindSubmit = bindFormSubmit(formEl, async () => {
    errorEl.style.display = 'none';
    const nombre = nombreInput.el.querySelector('input').value.trim();
    const contacto = contactoInput.el.querySelector('input').value.trim();
    const canal = canalInput.el.querySelector('input').value.trim();
    const promotora = promotoraInput.el.querySelector('input').value.trim();
    const personas = parseInt(personasInput.el.querySelector('input').value, 10);
    const notas = notasInput.el.querySelector('input').value.trim() || undefined;

    if (!nombre || !contacto || !canal || !promotora || isNaN(personas) || personas < 1) {
      errorEl.textContent = 'Complete los campos obligatorios. Personas debe ser al menos 1.';
      errorEl.style.display = 'block';
      return;
    }

    const payload = { nombre, contacto, canal, promotora, personas, notas };
    if (isEdit && statusSelect) {
      payload.estado = statusSelect.value;
    }

    try {
      await onSave(payload);
    } catch (err) {
      errorEl.textContent = (err && err.message) || 'Error al guardar interesado.';
      errorEl.style.display = 'block';
    }
  });

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    if (overlayEl.parentNode) {
      overlayEl.parentNode.removeChild(overlayEl);
    }
  }

  return { el: overlayEl, destroy };
}

/**
 * Creates an accessible modal dialog for converting an interest lead to ticket sales.
 * @param {{
 *   interest: any,
 *   ticketTypes: Array<{ nombre: string, precio: number, capacidad: number }>,
 *   onConvert: (data: { tipo_entrada: string, cantidad: number, medio_pago: string, estado_pago: string }) => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createConvertInterestDialog(props) {
  const { interest, ticketTypes = [], onConvert, onCancel } = props;

  const titleEl = createElement('h3', { class: 'events-dialog-title' }, ['Convertir a Venta de Entradas']);
  const infoEl = createElement('p', { class: 'events-dialog-subtitle' }, [
    `Cliente: ${interest.nombre} (${interest.contacto})`
  ]);

  const typeOptions = ticketTypes.map((t) =>
    createElement('option', { value: t.nombre }, [`${t.nombre} (S/ ${t.precio.toFixed(2)})`])
  );
  const typeSelect = createElement('select', { class: 'text-input events-select' }, typeOptions);
  const typeField = createElement('div', { class: 'text-input-field' }, [
    createElement('label', { class: 'text-input-label' }, ['Tipo de Entrada']),
    typeSelect,
  ]);

  const cantidadInput = createTextInput({
    label: 'Cantidad de Entradas',
    name: 'cantidad',
    type: 'number',
    min: '1',
    value: String(interest.personas || 1),
    required: true,
  });

  const medioSelect = createElement('select', { class: 'text-input events-select' }, [

    createElement('option', { value: 'Yape' }, ['Yape']),
    createElement('option', { value: 'Efectivo' }, ['Efectivo']),
    createElement('option', { value: 'Cortesía' }, ['Cortesía']),
  ]);
  medioSelect.value = 'Yape';
  const medioField = createElement('div', { class: 'text-input-field' }, [
    createElement('label', { class: 'text-input-label' }, ['Medio de Pago']),
    medioSelect,
  ]);

  const estadoSelect = createElement('select', { class: 'text-input events-select' }, [
    createElement('option', { value: 'Pagado' }, ['Pagado']),
    createElement('option', { value: 'Pendiente' }, ['Pendiente']),
  ]);
  estadoSelect.value = 'Pagado';
  const estadoField = createElement('div', { class: 'text-input-field' }, [
    createElement('label', { class: 'text-input-label' }, ['Estado de Pago']),
    estadoSelect,
  ]);

  medioSelect.addEventListener('change', () => {
    if (medioSelect.value === 'Cortesía') {
      estadoSelect.value = 'Pagado';
      estadoSelect.disabled = true;
    } else {
      estadoSelect.disabled = false;
    }
  });

  const errorEl = createElement('div', { class: 'events-form-error', style: 'display: none;' });

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: 'Confirmar Venta',
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'events-dialog-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement('form', { class: 'events-form' }, [
    infoEl,
    typeField,
    cantidadInput.el,
    medioField,
    estadoField,
    errorEl,
    actionsEl,
  ]);

  const dialogCard = createElement(
    'div',
    { class: 'events-dialog', role: 'dialog', 'aria-modal': 'true' },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  const unbindDismiss = bindDialogDismiss(overlayEl, onCancel);
  const unbindSubmit = bindFormSubmit(formEl, async () => {
    errorEl.style.display = 'none';
    const tipo_entrada = typeSelect.value;
    const cantidad = parseInt(cantidadInput.el.querySelector('input').value, 10);
    const medio_pago = medioSelect.value;
    const estado_pago = medio_pago === 'Cortesía' ? 'Pagado' : estadoSelect.value;

    if (!tipo_entrada || isNaN(cantidad) || cantidad < 1) {
      errorEl.textContent = 'Seleccione un tipo válido y cantidad de al menos 1.';
      errorEl.style.display = 'block';
      return;
    }

    try {
      await onConvert({ tipo_entrada, cantidad, medio_pago, estado_pago });
    } catch (err) {
      errorEl.textContent = (err && err.message) || 'Error al convertir interesado a venta.';
      errorEl.style.display = 'block';
    }
  });

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    if (overlayEl.parentNode) {
      overlayEl.parentNode.removeChild(overlayEl);
    }
  }

  return { el: overlayEl, destroy };
}

/**
 * Creates an accessible modal dialog for quick ticket sales.
 * Solicits buyer data, enforces single-option preselection rule, and upon
 * issuance renders cards with wa.me sharing links and public ticket links for each emitted ticket.
 *
 * @param {{
 *   eventName?: string,
 *   event?: { nombre?: string, fecha_hora?: string, lugar?: string } | null,
 *   ticketTypes: Array<{ nombre: string, precio: number, capacidad: number }>,
 *   onSell: (data: {
 *     nombre_comprador: string,
 *     contacto: string,
 *     tipo_entrada: string,
 *     cantidad: number,
 *     medio_pago: string,
 *     estado_pago: string
 *   }) => Promise<any> | any,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createSellTicketDialog(props) {
  const { eventName = '', event = null, ticketTypes = [], onSell, onCancel } = props;

  const titleEl = createElement('h3', { class: 'events-dialog-title' }, ['Venta Rápida de Entradas']);

  const compradorInput = createTextInput({
    label: 'Nombre del Comprador',
    name: 'nombre_comprador',
    placeholder: 'ej. Carlos Mendoza',
    required: true,
  });

  const contactoInput = createTextInput({
    label: 'Teléfono / WhatsApp (opcional)',
    name: 'contacto',
    placeholder: 'ej. 987654321',
    required: false,
  });

  // Preselection rule: preselect ONLY if there is exactly one valid ticket type option
  const hasSingleType = ticketTypes.length === 1;
  const typeOptions = [];
  if (!hasSingleType) {
    typeOptions.push(createElement('option', { value: '' }, ['Seleccione tipo de entrada...']));
  }
  ticketTypes.forEach((t) => {
    typeOptions.push(
      createElement('option', {
        value: t.nombre,
        selected: hasSingleType ? 'selected' : undefined,
      }, [`${t.nombre} (S/ ${Number(t.precio).toFixed(2)})`])
    );
  });

  const typeSelect = createElement('select', { class: 'text-input events-select' }, typeOptions);
  typeSelect.value = hasSingleType ? ticketTypes[0].nombre : '';

  const typeField = createElement('div', { class: 'text-input-field' }, [
    createElement('label', { class: 'text-input-label' }, ['Tipo de Entrada']),
    typeSelect,
  ]);

  const cantidadInput = createTextInput({
    label: 'Cantidad de Entradas',
    name: 'cantidad',
    type: 'number',
    min: '1',
    value: '1',
    required: true,
  });

  const medioSelect = createElement('select', { class: 'text-input events-select' }, [

    createElement('option', { value: 'Yape' }, ['Yape']),
    createElement('option', { value: 'Efectivo' }, ['Efectivo']),
    createElement('option', { value: 'Cortesía' }, ['Cortesía']),
  ]);
  medioSelect.value = 'Yape';
  const medioField = createElement('div', { class: 'text-input-field' }, [
    createElement('label', { class: 'text-input-label' }, ['Medio de Pago']),
    medioSelect,
  ]);

  const estadoSelect = createElement('select', { class: 'text-input events-select' }, [
    createElement('option', { value: 'Pagado' }, ['Pagado']),
    createElement('option', { value: 'Pendiente' }, ['Pendiente']),
  ]);
  estadoSelect.value = 'Pagado';
  const estadoField = createElement('div', { class: 'text-input-field' }, [
    createElement('label', { class: 'text-input-label' }, ['Estado de Pago']),
    estadoSelect,
  ]);

  medioSelect.addEventListener('change', () => {
    if (medioSelect.value === 'Cortesía') {
      estadoSelect.value = 'Pagado';
      estadoSelect.disabled = true;
    } else {
      estadoSelect.disabled = false;
    }
  });

  const errorEl = createElement('div', { class: 'events-form-error', style: 'display: none;' });

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: 'Emitir Entradas',
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'events-dialog-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement('form', { class: 'events-form' }, [
    compradorInput.el,
    contactoInput.el,
    typeField,
    cantidadInput.el,
    medioField,
    estadoField,
    errorEl,
    actionsEl,
  ]);

  const dialogCard = createElement(
    'div',
    { class: 'events-dialog', role: 'dialog', 'aria-modal': 'true' },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  function showIssuedTicketsView(issuedTickets, metadata) {
    titleEl.textContent = `¡Venta Realizada! (${issuedTickets.length} ${issuedTickets.length === 1 ? 'entrada emitida' : 'entradas emitidas'})`;

    const contactText = metadata.contacto ? ` (${metadata.contacto})` : '';
    const subtitle = createElement('p', { class: 'events-dialog-subtitle' }, [
      `Comprador: ${metadata.nombre_comprador}${contactText} - ${metadata.medio_pago} [${metadata.estado_pago}]`
    ]);

    const cards = issuedTickets.map((ticket, idx) => {
      const isPaid = ticket.estado_pago === 'Pagado';
      const badgeClass = isPaid ? 'events-badge-success' : 'events-badge-warning';

      const header = createElement('div', { class: 'events-quick-ticket-card-header' }, [
        createElement('strong', {}, [`Entrada #${idx + 1}: ${ticket.tipo_entrada}`]),
        createElement('span', { class: `events-badge ${badgeClass}` }, [ticket.estado_pago]),
      ]);

      const details = createElement('div', { class: 'events-quick-ticket-card-details' }, [
        createElement('span', {}, [`Precio: S/ ${Number(ticket.precio).toFixed(2)}`]),
      ]);

      const copyStatus = createElement('span', {
        class: 'events-quick-ticket-copy-status',
        role: 'status',
        'aria-live': 'polite',
      });

      const copyBtn = createButton({
        text: 'Copiar imagen',
        variant: 'primary',
        type: 'button',
        onClick: async () => {
          copyBtn.el.setAttribute('disabled', 'true');
          copyStatus.textContent = 'Preparando imagen…';
          try {
            const imagePromise = generarFotoEntrada(ticket, event || { nombre: eventName });
            if (
              typeof navigator !== 'undefined' &&
              navigator.clipboard &&
              typeof navigator.clipboard.write === 'function' &&
              typeof ClipboardItem !== 'undefined'
            ) {
              await navigator.clipboard.write([
                new ClipboardItem({ 'image/png': imagePromise }),
              ]);
              copyStatus.textContent = 'Imagen copiada. Ya puedes pegarla en el chat.';
            } else {
              const blob = await imagePromise;
              descargarBlob(blob, `entrada-${ticket.nombre_comprador || 'ticket'}.png`);
              copyStatus.textContent = 'El navegador no permite copiar imágenes; se descargó el PNG.';
            }
          } catch (err) {
            copyStatus.textContent = (err && err.message) || 'No se pudo copiar la imagen.';
          } finally {
            copyBtn.el.removeAttribute('disabled');
          }
        },
      });
      copyBtn.el.classList.add('events-quick-ticket-copy-btn');

      const publicTicketUrl = buildTicketPublicUrl(ticket.scan_token);
      const publicBtn = createElement('a', {
        href: publicTicketUrl,
        target: '_blank',
        class: 'btn btn-secondary btn-sm',
      }, ['Ver Ticket']);

      const cardActions = createElement('div', { class: 'events-quick-ticket-card-actions' }, [
        copyBtn.el,
        publicBtn,
      ]);

      return createElement('div', { class: 'events-quick-ticket-card' }, [
        header,
        details,
        cardActions,
        copyStatus,
      ]);
    });

    const listEl = createElement('div', { class: 'events-quick-tickets-list' }, cards);

    const closeBtn = createButton({
      text: 'Listo / Cerrar',
      variant: 'secondary',
      onClick: onCancel,
    });

    const resultActions = createElement('div', { class: 'events-dialog-actions' }, [closeBtn.el]);

    dialogCard.innerHTML = '';
    dialogCard.appendChild(titleEl);
    dialogCard.appendChild(subtitle);
    dialogCard.appendChild(listEl);
    dialogCard.appendChild(resultActions);
  }

  const unbindDismiss = bindDialogDismiss(overlayEl, onCancel);
  const unbindSubmit = bindFormSubmit(formEl, async () => {
    errorEl.style.display = 'none';
    const nombre_comprador = compradorInput.el.querySelector('input').value.trim();
    const contacto = contactoInput.el.querySelector('input').value.trim();
    const tipo_entrada = typeSelect.value;
    const cantidad = parseInt(cantidadInput.el.querySelector('input').value, 10);
    const medio_pago = medioSelect.value;
    const estado_pago = medio_pago === 'Cortesía' ? 'Pagado' : estadoSelect.value;

    if (!nombre_comprador) {
      errorEl.textContent = 'Ingresa el nombre del comprador.';
      errorEl.style.display = 'block';
      return;
    }
    if (!tipo_entrada) {
      errorEl.textContent = 'Selecciona un tipo de entrada.';
      errorEl.style.display = 'block';
      typeSelect.focus();
      return;
    }
    if (isNaN(cantidad) || cantidad < 1) {
      errorEl.textContent = 'La cantidad de entradas debe ser al menos 1.';
      errorEl.style.display = 'block';
      return;
    }

    try {
      submitBtn.el.setAttribute('disabled', 'true');
      const result = await onSell({
        nombre_comprador,
        contacto,
        tipo_entrada,
        cantidad,
        medio_pago,
        estado_pago,
      });

      if (Array.isArray(result) && result.length > 0) {
        showIssuedTicketsView(result, {
          nombre_comprador,
          contacto,
          tipo_entrada,
          medio_pago,
          estado_pago,
        });
      } else if (typeof onCancel === 'function') {
        onCancel();
      }
    } catch (err) {
      submitBtn.el.removeAttribute('disabled');
      errorEl.textContent = (err && err.message) || 'Error al emitir entradas.';
      errorEl.style.display = 'block';
    }
  });

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    if (overlayEl.parentNode) {
      overlayEl.parentNode.removeChild(overlayEl);
    }
  }

  return { el: overlayEl, destroy };
}

export const createQuickSaleDialog = createSellTicketDialog;
