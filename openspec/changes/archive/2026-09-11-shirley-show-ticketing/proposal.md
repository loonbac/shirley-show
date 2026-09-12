# Proposal: Ticketing y finanzas de eventos para Shirley Show

**Change:** `shirley-show-ticketing`  
**Status:** Proposed  
**Artifact store:** OpenSpec  
**Delivery:** `ask-on-risk`, presupuesto de revisión de 400 líneas por slice

## Intent

Portar al stack actual de Shirley Show la funcionalidad operativa del sistema de eventos en producción, sin trasladar su diseño visual ni desplazar el flujo principal de repertorios. La aplicación incorporará una segunda sección, **Eventos**, desde la que Shirley y su equipo podrán administrar eventos, vender y entregar entradas, hacer seguimiento de interesados, registrar gastos, consultar resultados y validar accesos en puerta.

El cambio busca concentrar en una sola aplicación el trabajo que hoy exige controles separados o manuales, manteniendo una operación sencilla: sin cuentas de usuario, sin pasarela de pagos y sin integración servidor-a-servidor con WhatsApp. Ressy también podrá ejecutar las operaciones frecuentes mediante ocho herramientas nuevas.

## Resultado de producto

Al completar el cambio:

- **Repertorios** seguirá siendo la primera sección y la ruta inicial de la aplicación.
- **Eventos** aparecerá como una segunda sección hermana, adaptada al sistema visual y a los patrones de interacción existentes de Shirley Show.
- El equipo podrá gestionar el ciclo completo de un evento: creación, edición, eliminación, tipos de entrada, afiche y finalización o reapertura.
- Una venta de varias entradas emitirá una fila y credenciales únicas por cada entrada, en una operación atómica.
- Las reservas pendientes podrán entregarse al comprador sin exponer un QR utilizable; confirmar el pago habilitará su QR.
- Los interesados podrán registrarse, actualizarse y pasarse al flujo de venta sin duplicar reglas de emisión.
- Los gastos y el resumen permitirán conocer cobros, pendientes, egresos, ganancia y ocupación, incluyendo el desglose de cobros por Yape y Efectivo.
- El personal de puerta podrá distinguir de forma inmediata una entrada válida, ya utilizada o inválida.
- El comprador podrá abrir su entrada pública desde el teléfono y el equipo podrá compartirla mediante enlaces `wa.me` generados por el navegador.

## Alcance funcional

### 1. Eventos

- CRUD de eventos con nombre, fecha y hora, lugar y una colección validada de tipos de entrada.
- Cada tipo de entrada define, como mínimo, nombre, precio y capacidad.
- Finalización y reapertura explícitas sin perder la información histórica del evento.
- Carga, reemplazo, consulta y presentación de un afiche opcional, con límite de 10 MiB y almacenamiento controlado.
- Listado y detalle con estados activos/finalizados y datos resumidos relevantes.

### 2. Entradas, pagos y entrega

- Venta multi-fila: una cantidad `N` crea exactamente `N` entradas individuales dentro de una única transacción.
- Cada entrada recibe un `scan_token` público para su URL y un `codigo_qr` secreto e independiente para validación en puerta.
- Medios de pago admitidos: Yape, Efectivo y Cortesía; estados de pago: Pagado y Pendiente.
- Confirmación manual de pago para pasar una entrada pendiente a pagada.
- El backend no generará ni expondrá el QR mientras la entrada permanezca pendiente.
- Consulta, búsqueda, edición y eliminación de entradas según la paridad funcional del sistema de producción.
- Vista pública por `scan_token`, optimizada para móvil, con datos del evento y estado de pago.
- Enlaces `wa.me` construidos en el frontend para compartir la entrada pagada o las instrucciones de una reserva pendiente.

### 3. Interesados

- CRUD de interesados con nombre, contacto, canal, número de personas, promotora y estado.
- El número de personas será al menos uno y participará en el aforo proyectado cuando corresponda al estado confirmado.
- Acción de pasar un interesado a venta usando el mismo caso de uso transaccional de venta de entradas; no se implementará una segunda lógica de emisión.
- La especificación detallará la transición de estado y el comportamiento ante una conversión parcial o fallida para evitar ventas duplicadas.

### 4. Gastos y resumen

- CRUD de gastos con monto, descripción, fecha y una categoría perteneciente al catálogo cerrado de producción.
- Resumen con las siete métricas de paridad del sistema de producción, incluido como mínimo Cobrado, Pendiente, Gastos, Ganancia neta y ocupación/aforo, más el desglose de cobros por Yape y Efectivo.
- Invariantes financieras:
  - `Cobrado = suma(precio)` de entradas pagadas.
  - `Pendiente = suma(precio)` de entradas pendientes.
  - `Gastos = suma(monto)` de gastos del evento.
  - `Ganancia neta = Cobrado - Gastos`.
- El aforo proyectado contará cada entrada emitida una sola vez y añadirá las personas de interesados confirmados; los detalles exactos de las siete métricas se fijarán contra el comportamiento de producción en la especificación, sin inventar indicadores nuevos.

### 5. Check-in en puerta

- Ruta de puerta por evento con entrada manual y soporte para lectura del valor contenido en el QR.
- La validación usará exclusivamente `codigo_qr`, nunca el `scan_token` de la URL pública.
- Resultado inequívoco en tres estados: válida, ya usada o inválida.
- Una validación exitosa registrará el ingreso de forma atómica para impedir el doble uso.
- Entradas pendientes, códigos de otro evento y valores desconocidos no producirán un check-in válido.

### 6. Ressy

Se añadirán exactamente ocho herramientas operativas:

1. `crear_evento`
2. `listar_eventos`
3. `vender_entradas`
4. `confirmar_pago_entrada`
5. `registrar_gasto`
6. `resumen_evento`
7. `registrar_interesado`
8. `checkin_entrada`

Las herramientas reutilizarán los mismos casos de uso que la API, no accederán directamente a detalles internos de SQLite y conservarán las validaciones de dominio. Ressy distinguirá dinero cobrado de reservas pendientes, solicitará los datos ambiguos antes de vender y devolverá enlaces aptos para compartir cuando corresponda.

## Reglas e invariantes principales

1. Una venta de `N` entradas es todo-o-nada y genera `N` filas con identificadores y tokens únicos.
2. El token público de visualización y el código secreto de puerta cumplen funciones distintas y no son intercambiables.
3. Ninguna respuesta pública o administrativa debe incluir un QR renderizable para una entrada pendiente.
4. La confirmación de pago no sustituye el check-in ni marca una entrada como utilizada.
5. Un check-in exitoso solo puede ocurrir una vez y debe persistir su hora de ingreso.
6. La ganancia siempre se deriva de cobros efectivos menos gastos; los montos pendientes no cuentan como cobro.
7. La conversión de un interesado usa el flujo normal de venta y debe evitar duplicados ante reintentos o fallos parciales.
8. El cierre de un evento es reversible mediante reapertura y no elimina sus datos.
9. Las capacidades y categorías se validan en las fronteras del backend; la UI no será la única barrera.

## Áreas afectadas

### Backend Rust/Axum

- Nuevo módulo cohesivo de ticketing/eventos con dominio, casos de uso y contratos de persistencia.
- Adaptador SQLite y migración para eventos, entradas, interesados y gastos.
- Rutas Axum administrativas, públicas, de afiche, resumen y check-in.
- Generación de QR SVG y tokens criptográficamente impredecibles mediante dependencias adecuadas del ecosistema Rust.
- Almacenamiento de afiches bajo un propietario explícito y con validación de tamaño, tipo y ruta.

### Frontend vanilla JS

- Navegación principal y nuevas rutas hash para catálogo y detalle de eventos, entrada pública y modo puerta.
- Vistas de resumen, entradas, interesados y gastos, con estados de carga, vacío y error.
- Formularios de venta, confirmación de pago, conversión de interesados y registro de gastos.
- Presentación responsive del ticket, QR condicionado al pago y generación local de enlaces `wa.me`.
- Componentes y comportamientos permanecerán dentro del módulo de eventos salvo que exista un consumidor compartido real.

### Ressy

- Esquemas de ocho herramientas, despacho a casos de uso y actualización acotada del prompt de sistema.
- Respuestas orientadas a la operación y protección contra acciones ambiguas o inconsistentes.

### Datos y operación

- Nueva migración SQLite y nuevos archivos de afiche bajo `data/posters/` o la ubicación equivalente acordada por el adaptador.
- No se tocará `.run/` ni se iniciarán procesos sobre el puerto `:3100` durante desarrollo o verificación.

## Compatibilidad y límites arquitectónicos

- Se portará **funcionalidad, no diseño**: no se copiarán componentes, estilos ni estructura visual del frontend de producción.
- El cambio ampliará el monolito modular existente; no creará servicios independientes ni buses globales.
- API, frontend y herramientas de Ressy compartirán casos de uso y reglas de dominio.
- La vista pública tendrá una superficie de datos mínima y nunca expondrá secretos internos distintos del QR pagado que el comprador debe presentar.
- Repertorios conservará su comportamiento y prioridad actuales.

## Fuera de alcance

- Autenticación, autorización, usuarios, roles o permisos.
- Whatsmeow, bots de WhatsApp, webhooks o envío automatizado de mensajes; solo enlaces `https://wa.me/...`.
- Pasarelas de pago, conciliación bancaria automática o confirmación automática de Yape/Efectivo.
- Rediseño visual de Shirley Show o copia del diseño del sistema de producción.
- Reservas numeradas, selección de asientos, descuentos, reembolsos o facturación electrónica.
- Cambios funcionales al módulo de repertorios más allá de integrar el selector de sección.

## Estrategia de entrega

La implementación se dividirá en seis slices revisables:

1. **Base de eventos y finanzas:** migración, dominio y persistencia de eventos, gastos y cálculos de resumen.
2. **Entradas y puerta:** venta transaccional multi-fila, doble token, confirmación de pago, QR público condicionado y check-in.
3. **Interesados y afiches:** CRUD, pase a venta, almacenamiento seguro de afiches, finalización/reapertura y endpoints administrativos restantes.
4. **Ressy:** ocho tools, despacho a casos de uso, contexto conversacional y pruebas de contratos.
5. **Frontend de administración:** navegación, catálogo, detalle y pestañas de resumen, entradas, interesados y gastos.
6. **Frontend operativo/público:** venta rápida, `wa.me`, ticket público y modo puerta.

Cada slice buscará mantenerse dentro del presupuesto de 400 líneas cambiadas. Con la estrategia `ask-on-risk`, cualquier slice que previsiblemente supere el presupuesto o mezcle áreas difíciles de revisar deberá detenerse antes de implementarse para acordar encadenamiento o ajuste de alcance; no se asumirá una excepción de tamaño.

## Riesgos y mitigaciones

| Riesgo | Impacto | Mitigación propuesta |
|---|---|---|
| Exposición o confusión entre tokens | Una URL pública podría convertirse indebidamente en credencial de acceso | Tipos/contratos separados, respuesta pública mínima y check-in que acepte solo `codigo_qr` |
| QR visible antes del pago | Acceso de reservas no cobradas | Condición aplicada en el backend y cubierta por pruebas; la UI solo representa la respuesta segura |
| Venta parcial de varias entradas | Cobros y aforo inconsistentes | Una única transacción SQLite con rollback total ante cualquier error |
| Doble check-in concurrente | Dos accesos con una entrada | Actualización condicional y atómica del estado de ingreso |
| Conversión duplicada de interesados | Sobreventa o entradas repetidas | Operación coordinada con reglas de transición e idempotencia/reintento definidas en spec y diseño |
| Cálculos financieros divergentes | Decisiones operativas erróneas | Un único propietario de las fórmulas y pruebas de consultas con pagos, cortesías, pendientes y gastos |
| Afiche malicioso o excesivo | Path traversal, contenido inesperado o consumo de disco | Límite de 10 MiB, nombre generado por servidor, tipos permitidos y ciclo de reemplazo/eliminación explícito |
| Ausencia de autenticación | Endpoints administrativos accesibles en un despliegue expuesto | Mantener la premisa de despliegue confiable, no presentar la ausencia de auth como seguridad y documentar el límite operativo |
| Expansión transversal del cambio | Review deficiente y regresiones en repertorios | Seis slices acotados, contratos públicos pequeños y pausa obligatoria ante riesgo de superar 400 líneas |

## Rollback

- Cada slice deberá poder revertirse de forma independiente antes de comenzar el siguiente cuando no exista una dependencia desplegada.
- La navegación a **Eventos**, las rutas API y las tools de Ressy podrán retirarse sin modificar los datos de repertorios.
- Si una versión debe revertirse después de aplicar la migración, se restaurará el binario/frontend anterior y se conservarán las tablas de ticketing inactivas para evitar pérdida de ventas, check-ins o gastos. No se ejecutará un `DROP TABLE` automático.
- Los afiches permanecerán en disco durante un rollback de aplicación y solo se eliminarán mediante una operación administrativa deliberada.
- Si la función de QR o check-in presenta un defecto, se deshabilitará temporalmente la ruta de puerta afectada antes que relajar la validación o aceptar `scan_token` como sustituto.
- Cualquier migración destructiva o limpieza de datos requerirá respaldo y autorización humana independiente.

## Criterios de éxito

1. Repertorios continúa siendo la vista inicial y sus flujos existentes no cambian, mientras Eventos es accesible como segunda sección.
2. El CRUD de eventos, tipos de entrada, afiche y transición finalizar/reabrir funciona de extremo a extremo.
3. Vender una cantidad `N` produce exactamente `N` entradas únicas o ninguna si falla cualquier inserción.
4. Una entrada pendiente se puede consultar públicamente, pero ninguna respuesta contiene su QR; tras confirmar el pago, el ticket público muestra un QR que codifica solo `codigo_qr`.
5. Puerta acepta una entrada pagada del evento una sola vez y distingue correctamente válida, ya usada e inválida, incluyendo pendientes y códigos de otro evento.
6. Un interesado puede pasar a venta sin duplicar la lógica de emisión ni dejar una conversión parcial visible.
7. Los gastos respetan su catálogo de categorías y el resumen reproduce las siete métricas de producción, el desglose Yape/Efectivo y `Ganancia neta = Cobrado - Gastos`.
8. Los enlaces `wa.me` contienen texto y URL pública correctamente codificados y no requieren servicios de WhatsApp en el backend.
9. Las ocho herramientas de Ressy ejecutan los mismos casos de uso que la API y manejan datos ambiguos sin crear registros prematuros.
10. Las pruebas cubren invariantes de transacción, pago/QR, doble token, check-in, conversión y finanzas; la validación se ejecuta sin usar `.run/` ni el puerto `:3100`.
11. No se introduce autenticación, pasarela de pago, Whatsmeow ni un rediseño del producto como efecto secundario.

## Decisiones confirmadas para fases siguientes

- El objetivo es paridad funcional con producción, no paridad visual.
- El alcance se entrega en seis slices y se protege con presupuesto de revisión de 400 líneas bajo `ask-on-risk`.
- La especificación debe fijar contra producción el contrato exacto de las siete métricas, las transiciones del pase de interesado a venta y las respuestas mínimas de ticket público/check-in.
- La implementación seguirá TDD estricto con el runner detectado (`cargo test`) y pruebas herméticas con SQLite temporal o en memoria.
