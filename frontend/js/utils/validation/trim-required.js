/**
 * Validates that a string value is not empty after trimming.
 * @param {string|null|undefined} value 
 * @param {string} [fieldName='Este campo'] 
 * @returns {{ valid: boolean, value: string, error?: string }}
 */
export function trimRequired(value, fieldName = 'Este campo') {
  if (value === null || value === undefined) {
    return {
      valid: false,
      value: '',
      error: `${fieldName} es obligatorio.`,
    };
  }

  const trimmed = String(value).trim();
  if (trimmed.length === 0) {
    return {
      valid: false,
      value: '',
      error: `${fieldName} no puede estar vacío.`,
    };
  }

  return {
    valid: true,
    value: trimmed,
  };
}
