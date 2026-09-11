import { requestJson, ApiError } from './request-json.js';

export const MAX_AUDIO_BYTES = 100 * 1024 * 1024; // 104,857,600 bytes (100 MiB)
export const ALLOWED_AUDIO_EXTENSIONS = ['.mp3', '.wav', '.m4a'];

/**
 * Validates an audio attachment against size and extension constraints.
 * 
 * @param {{ name: string, size: number }} file
 * @returns {{ valid: true } | { valid: false, error: { code: string, message: string } }}
 */
export function validateAudioAttachment(file) {
  if (!file) {
    return { valid: true };
  }

  if (file.size > MAX_AUDIO_BYTES) {
    return {
      valid: false,
      error: {
        code: 'file_too_large',
        message: 'El archivo de audio supera el límite máximo de 100 MiB.',
      },
    };
  }

  const name = (file.name || '').toLowerCase();
  const hasValidExt = ALLOWED_AUDIO_EXTENSIONS.some((ext) => name.endsWith(ext));

  if (!hasValidExt) {
    return {
      valid: false,
      error: {
        code: 'unsupported_format',
        message: 'Formato de audio no soportado. Se permiten únicamente archivos .mp3, .wav y .m4a.',
      },
    };
  }

  return { valid: true };
}

/**
 * Dispatches a chat message to Shirley AI Agent backend.
 * Uses JSON for text-only messages, and FormData for turns with an attached audio file.
 *
 * @param {{
 *   message: string,
 *   history?: Array<{ role: 'user' | 'assistant', content: string }>,
 *   presentationId?: string | null,
 *   stagedAudioId?: string | null,
 *   attachment?: File | null,
 *   signal?: AbortSignal,
 * }} options
 * @returns {Promise<{
 *   message: string,
 *   actions_taken: Array<any>,
 *   active_presentation_id: string | null,
 *   notice: { code: string, retryable: boolean } | null,
 * }>}
 */
export async function sendAgentChatMessage(options) {
  const {
    message,
    history = [],
    presentationId = null,
    stagedAudioId = null,
    attachment = null,
    signal,
  } = options;

  if (attachment) {
    const validation = validateAudioAttachment(attachment);
    if (!validation.valid) {
      throw new ApiError(400, validation.error.code, validation.error.message, 'file');
    }

    const formData = new FormData();
    formData.append('message', message || '');
    formData.append('history', JSON.stringify(history));
    if (presentationId) {
      formData.append('presentation_id', presentationId);
    }
    if (stagedAudioId) {
      formData.append('staged_audio_id', stagedAudioId);
    }
    formData.append('file', attachment);

    try {
      return await requestJson('/api/agent/chat', {
        method: 'POST',
        body: formData,
        signal,
      });
    } catch (err) {
      throw classifyChatError(err);
    }
  }

  const jsonBody = {
    message: message || '',
    history,
    presentation_id: presentationId || null,
    staged_audio_id: stagedAudioId || null,
  };

  try {
    return await requestJson('/api/agent/chat', {
      method: 'POST',
      body: jsonBody,
      signal,
    });
  } catch (err) {
    throw classifyChatError(err);
  }
}

/**
 * Classifies network and HTTP errors into standardized error descriptors.
 * @param {any} err
 * @returns {ApiError & { retryable?: boolean }}
 */
function classifyChatError(err) {
  if (err instanceof ApiError) {
    let retryable = true;
    if (err.code === 'provider_auth' || err.code === 'agent_unconfigured' || err.code === 'invalid_request') {
      retryable = false;
    }
    err.retryable = retryable;
    return err;
  }

  return new ApiError(0, 'network_error', 'No se pudo conectar con el servidor.');
}
