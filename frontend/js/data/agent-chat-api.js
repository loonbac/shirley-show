import { requestJson, ApiError } from './request-json.js';

export const MAX_AUDIO_BYTES = 100 * 1024 * 1024; // 104,857,600 bytes (100 MiB)
export const MAX_POSTER_BYTES = 10 * 1024 * 1024; // 10,485,760 bytes (10 MiB)
export const ALLOWED_AUDIO_EXTENSIONS = ['.mp3', '.wav', '.m4a'];
export const ALLOWED_IMAGE_EXTENSIONS = ['.jpg', '.jpeg', '.png', '.webp'];

/**
 * Validates an attachment against audio or image constraints.
 * 
 * @param {{ name: string, size: number, type?: string }} file
 * @returns {{ valid: true, kind: 'audio' | 'image' | null } | { valid: false, error: { code: string, message: string } }}
 */
export function validateAgentAttachment(file) {
  if (!file) {
    return { valid: true, kind: null };
  }

  const name = (file.name || '').toLowerCase();
  const type = (file.type || '').toLowerCase();

  const isImage =
    ALLOWED_IMAGE_EXTENSIONS.some((ext) => name.endsWith(ext)) ||
    type.startsWith('image/') ||
    type === 'image/jpeg' ||
    type === 'image/png' ||
    type === 'image/webp';

  const isAudio =
    ALLOWED_AUDIO_EXTENSIONS.some((ext) => name.endsWith(ext)) ||
    type.startsWith('audio/') ||
    type === 'audio/mpeg' ||
    type === 'audio/wav' ||
    type === 'audio/x-m4a';

  if (isImage) {
    if (file.size > MAX_POSTER_BYTES) {
      return {
        valid: false,
        error: {
          code: 'file_too_large',
          message: 'El archivo de imagen supera el límite máximo de 10 MiB.',
        },
      };
    }
    return { valid: true, kind: 'image' };
  }

  if (isAudio) {
    if (file.size > MAX_AUDIO_BYTES) {
      return {
        valid: false,
        error: {
          code: 'file_too_large',
          message: 'El archivo de audio supera el límite máximo de 100 MiB.',
        },
      };
    }
    return { valid: true, kind: 'audio' };
  }

  return {
    valid: false,
    error: {
      code: 'unsupported_format',
      message: 'Formato no soportado. Se admiten archivos de audio (.mp3, .wav, .m4a) o imágenes (.jpg, .jpeg, .png, .webp).',
    },
  };
}

/**
 * Validates an audio attachment against size and extension constraints (backward-compatible wrapper).
 * 
 * @param {{ name: string, size: number }} file
 * @returns {{ valid: true } | { valid: false, error: { code: string, message: string } }}
 */
export function validateAudioAttachment(file) {
  const res = validateAgentAttachment(file);
  if (!res.valid) {
    return res;
  }
  if (res.kind === 'image') {
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
 * Dispatches a chat message to Ressy AI Agent backend.
 * Uses JSON for text-only messages, and FormData for turns with an attached audio file.
 *
 * @param {{
 *   message: string,
 *   history?: Array<{ role: 'user' | 'assistant', content: string }>,
 *   presentationId?: string | null,
 *   eventId?: string | null,
 *   stagedAudioId?: string | null,
 *   attachment?: File | null,
 *   signal?: AbortSignal,
 * }} options
 * @returns {Promise<{
 *   message: string,
 *   actions_taken: Array<any>,
 *   active_presentation_id: string | null,
 *   active_event_id: string | null,
 *   notice: { code: string, retryable: boolean } | null,
 * }>}
 */
export async function sendAgentChatMessage(options) {
  const {
    message,
    history = [],
    presentationId = null,
    eventId = null,
    stagedAudioId = null,
    conversationId = null,
    sessionId = null,
    attachment = null,
    signal,
  } = options;

  const activeSessionId = sessionId || options.session_id || null;

  if (attachment) {
    const validation = validateAgentAttachment(attachment);
    if (!validation.valid) {
      throw new ApiError(400, validation.error.code, validation.error.message, 'file');
    }

    const formData = new FormData();
    formData.append('message', message || '');
    formData.append('history', JSON.stringify(history));
    if (presentationId) {
      formData.append('presentation_id', presentationId);
    }
    if (eventId) {
      formData.append('event_id', eventId);
    }
    if (stagedAudioId) {
      formData.append('staged_audio_id', stagedAudioId);
    }
    if (conversationId) {
      formData.append('conversation_id', conversationId);
    }
    if (activeSessionId) {
      formData.append('session_id', activeSessionId);
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
    event_id: eventId || null,
    staged_audio_id: stagedAudioId || null,
    ...(conversationId ? { conversation_id: conversationId } : {}),
    ...(activeSessionId ? { session_id: activeSessionId } : {}),
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
 * Probes the GPU processor presence state.
 * Cheap, read-only 3-second probe without side effects.
 *
 * @param {{ signal?: AbortSignal }} [options={}]
 * @returns {Promise<{ mode: 'local' | 'remote', pc_online: boolean }>}
 */
export async function getGpuStatus(options = {}) {
  const { signal } = options;
  return await requestJson('/api/agent/gpu-status', {
    method: 'GET',
    signal,
  });
}

/**
 * Resolves the appropriate pending typing indicator label for a vocal separation
 * intent based on genuine GPU presence status.
 *
 * @param {{ mode?: string, pc_online?: boolean } | null} [gpuStatus=null]
 * @returns {'esperando la PC de Joshua…' | 'separando voces…'}
 */
export function resolveVocalPendingLabel(gpuStatus) {
  if (gpuStatus && gpuStatus.mode === 'remote' && !gpuStatus.pc_online) {
    return 'esperando la PC de Joshua…';
  }
  return 'separando voces…';
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
