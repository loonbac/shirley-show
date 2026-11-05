import { test, expect } from '@playwright/test';

test.describe('Validación de Formularios de Eventos E2E', () => {

  test('Validación: muestra error amigable si los campos obligatorios están vacíos', async ({ page }) => {
    await page.goto('/panel#/eventos');
    await page.locator('button:has-text("Nuevo Evento"), button:has-text("Crear Primer Evento")').first().click();

    const dialog = page.locator('.events-dialog').first();
    await expect(dialog).toBeVisible();

    // Intentar enviar sin nombre
    await dialog.locator('button[type="submit"]').click();

    const errorEl = dialog.locator('.events-form-error');
    await expect(errorEl).toBeVisible();
    await expect(errorEl).toContainText('El nombre del evento es requerido');
  });

  test('Validación: muestra error si la fecha ingresada es inválida', async ({ page }) => {
    await page.goto('/panel#/eventos');
    await page.locator('button:has-text("Nuevo Evento"), button:has-text("Crear Primer Evento")').first().click();

    const dialog = page.locator('.events-dialog').first();
    await expect(dialog).toBeVisible();

    await dialog.locator('input[name="nombre"]').fill('Evento Fecha Invalida');
    await dialog.locator('input[name="lugar"]').fill('Local');

    // Manipular o ingresar fecha no válida
    await page.evaluate(() => {
      const input = document.querySelector('input[name="fecha_hora"]');
      if (input) {
        input.type = 'text'; // simula navegador sin picker o escritura manual
        input.value = 'fecha-totalmente-invalida';
      }
    });

    await dialog.locator('button[type="submit"]').click();

    const errorEl = dialog.locator('.events-form-error');
    await expect(errorEl).toBeVisible();
    await expect(errorEl).toContainText('Fecha u hora inválida');
  });

  test('Soporte de fecha humana: acepta fechas escritas en formato AAAA-MM-DD HH:MM', async ({ page }) => {
    await page.goto('/panel#/eventos');
    await page.locator('button:has-text("Nuevo Evento"), button:has-text("Crear Primer Evento")').first().click();

    const dialog = page.locator('.events-dialog').first();
    await expect(dialog).toBeVisible();

    await dialog.locator('input[name="nombre"]').fill('Evento Fecha Escrita Manual');
    await dialog.locator('input[name="lugar"]').fill('Teatro Municipal');

    // Escribir fecha con espacio en lugar de 'T'
    await page.evaluate(() => {
      const input = document.querySelector('input[name="fecha_hora"]');
      if (input) {
        input.type = 'text';
        input.value = '2026-12-20 20:30';
      }
    });

    await dialog.locator('button[type="submit"]').click();

    // Debe guardarse exitosamente
    await expect(dialog).not.toBeVisible();
    await expect(page.locator('.events-card').filter({ hasText: 'Evento Fecha Escrita Manual' })).toBeVisible();
  });

  test('Validación de entradas: no permite tipos de entrada con capacidad 0 o nombre vacío', async ({ page }) => {
    await page.goto('/panel#/eventos');
    await page.locator('button:has-text("Nuevo Evento"), button:has-text("Crear Primer Evento")').first().click();

    const dialog = page.locator('.events-dialog').first();
    await expect(dialog).toBeVisible();

    await dialog.locator('input[name="nombre"]').fill('Evento Capacidad Cero');
    await dialog.locator('input[name="lugar"]').fill('Local');
    await dialog.locator('input[name="fecha_hora"]').fill('2026-11-20T20:00');

    // Vaciar el nombre del tipo de entrada
    const generalRow = dialog.locator('.events-type-row').first();
    await generalRow.locator('.events-type-name').fill('');

    await dialog.locator('button[type="submit"]').click();

    const errorEl = dialog.locator('.events-form-error');
    await expect(errorEl).toBeVisible();
    await expect(errorEl).toContainText('nombre del tipo de entrada no puede estar vacío');
  });

});
