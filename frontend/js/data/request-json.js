/**
 * Custom error class capturing structured backend error responses.
 */
export class ApiError extends Error {
  /**
   * @param {number} status 
   * @param {string} code 
   * @param {string} message 
   * @param {string} [field] 
   */
  constructor(status, code, message, field) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
    this.field = field;
  }
}

/**
 * Generic fetch wrapper for JSON and FormData endpoints.
 * @template T
 * @param {string} url 
 * @param {RequestInit & { body?: any }} [options={}] 
 * @returns {Promise<T>}
 */
export async function requestJson(url, options = {}) {
  const { headers = {}, body, ...rest } = options;
  const isFormData = body instanceof FormData;

  const reqHeaders = new Headers(headers);
  let reqBody = body;

  if (!isFormData && body !== undefined && body !== null && typeof body === 'object') {
    reqHeaders.set('Content-Type', 'application/json');
    reqBody = JSON.stringify(body);
  }

  let response;
  try {
    response = await fetch(url, {
      ...rest,
      headers: reqHeaders,
      body: reqBody,
    });
  } catch (netErr) {
    throw new ApiError(0, 'network_error', 'No se pudo conectar con el servidor.');
  }

  let responseData = null;
  const contentType = response.headers.get('content-type') || '';
  if (contentType.includes('application/json')) {
    try {
      responseData = await response.json();
    } catch {
      responseData = null;
    }
  }

  if (!response.ok) {
    const errorObj = responseData && responseData.error ? responseData.error : {};
    const code = errorObj.code || `http_${response.status}`;
    const message = errorObj.message || `Error ${response.status}: ${response.statusText}`;
    const field = errorObj.field;

    throw new ApiError(response.status, code, message, field);
  }

  return responseData;
}
