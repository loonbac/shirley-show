/**
 * Headless Camera QR Scanner with safe lifecycle management.
 * Leverages native browser BarcodeDetector API with graceful fallback to manual input.
 * Guarantees camera track release on stop/teardown.
 */

/**
 * Headless Camera QR Scanner with safe lifecycle management.
 * Leverages native browser BarcodeDetector API when available,
 * with universal fallback to canvas snapshots + jsQR library.
 * Guarantees camera track release on stop/teardown.
 */

/**
 * Checks if the browser natively supports BarcodeDetector with QR code format.
 * @returns {boolean}
 */
export function isBarcodeDetectorSupported() {
  if (typeof window === 'undefined') return false;
  if (!('BarcodeDetector' in window)) return false;
  if (typeof navigator === 'undefined' || !navigator.mediaDevices || typeof navigator.mediaDevices.getUserMedia !== 'function') return false;
  return true;
}

/**
 * Checks if camera capture is supported via mediaDevices.getUserMedia.
 * @returns {boolean}
 */
export function isCameraSupported() {
  if (typeof window === 'undefined' || typeof navigator === 'undefined') return false;
  if (!navigator.mediaDevices || typeof navigator.mediaDevices.getUserMedia !== 'function') return false;
  return true;
}

/**
 * Resolves the jsQR library from global scope if available.
 * Prefiere la forma UMD oficial (`module.exports.default` interop); si el
 * vendor actual es el build oficial, `window.jsQR` puede no ser función
 * directa y hay que resolver `.default`.
 * @returns {((data: Uint8ClampedArray, width: number, height: number, options?: any) => any) | null}
 */
export function getJsQR() {
  const candidates = [];
  if (typeof globalThis !== 'undefined') {
    candidates.push(globalThis.jsQR);
    candidates.push(globalThis.jsqr);
  }
  if (typeof window !== 'undefined') {
    candidates.push(window.jsQR);
    candidates.push(window.jsqr);
  }
  if (typeof self !== 'undefined' && typeof self.jsQR !== 'undefined') {
    candidates.push(self.jsQR);
  }
  for (const c of candidates) {
    if (typeof c === 'function') return c;
    if (c && typeof c.default === 'function') return c.default;
  }
  return null;
}

/**
 * Decodifica un frame RGBA probando frame completo, recorte central y
 * mitad de resolución. Definida a nivel de módulo para que el loop de
 * escaneo y los tests compartan el mismo pipeline (sin duplicación).
 * @param {(data: Uint8ClampedArray, w: number, h: number, opts?: any) => any} jsQRFn
   * @param {Uint8ClampedArray} data frame RGBA contiguo
 * @param {number} width ancho del frame
 * @param {number} height alto del frame
 * @param {{ onDecodeError?: (err: unknown) => void }} [hooks={}] receptor de errores del vendor (reporta y sigue, nunca silencia)
 * @returns {string|null} payload recortado o null si ningún intento decodifica
 */
export function decodeFrameWithFallbacks(jsQRFn, data, width, height, hooks = {}) {
  const { onDecodeError = () => {} } = hooks;
  if (typeof jsQRFn !== 'function' || !data || width <= 0 || height <= 0) return null;
  // Full frame primero (QR cercano), luego centro recortado (QR lejano).
  const attempts = [{ data, w: width, h: height }];
  const cropFrac = 0.6;
  if (width > 160 && height > 160) {
    const cw = Math.floor(width * cropFrac);
    const ch = Math.floor(height * cropFrac);
    const ox = Math.floor((width - cw) / 2);
    const oy = Math.floor((height - ch) / 2);
    attempts.push({ data: cropCenter(data, width, height, ox, oy, cw, ch), w: cw, h: ch });
  }
  for (const a of attempts) {
    let code = null;
    try {
      code = jsQRFn(a.data, a.w, a.h, { inversionAttempts: 'attemptBoth' });
    } catch (err) {
      onDecodeError(err);
    }
    if (!code) {
      try {
        code = downscaleDecode(jsQRFn, a.data, a.w, a.h);
      } catch (err) {
        onDecodeError(err);
      }
    }
    if (code && code.data) {
      const rawValue = String(code.data).trim();
      if (rawValue) return rawValue;
    }
  }
  return null;
}

/**
 * Extrae el rectángulo central (ox,oy,cw,ch) como buffer RGBA contiguo.
 */
export function cropCenter(data, width, height, ox, oy, cw, ch) {
  const out = new Uint8ClampedArray(cw * ch * 4);
  for (let y = 0; y < ch; y++) {
    const srcStart = ((oy + y) * width + ox) * 4;
    out.set(data.subarray(srcStart, srcStart + cw * 4), y * cw * 4);
  }
  return out;
}

/**
 * Reintento a mitad de resolución para QR lejanos/pequeños.
 * Devuelve el resultado de jsQR o null.
 */
export function downscaleDecode(jsQRFn, data, width, height) {
  if (width < 320 || height < 320) return null;
  const sw = Math.floor(width / 2);
  const sh = Math.floor(height / 2);
  const small = new Uint8ClampedArray(sw * sh * 4);
  for (let y = 0; y < sh; y++) {
    for (let x = 0; x < sw; x++) {
      const si = ((y * 2) * width + x * 2) * 4;
      const di = (y * sw + x) * 4;
      small[di] = data[si];
      small[di + 1] = data[si + 1];
      small[di + 2] = data[si + 2];
      small[di + 3] = data[si + 3];
    }
  }
  return jsQRFn(small, sw, sh, { inversionAttempts: 'attemptBoth' });
}

/**
 * Creates a headless camera QR scanner controller.
 *
 * @param {{
 *   videoEl: HTMLVideoElement,
 *   onScan: (code: string) => void,
 *   onError?: (error: Error) => void,
 *   scanIntervalMs?: number,
 * }} options 
 * @returns {{
 *   start: () => Promise<void>,
 *   stop: () => void,
 *   isScanning: () => boolean,
 *   destroy: () => void,
 * }}
 */
export function createCameraQrScanner(options) {
  const {
    videoEl,
    onScan,
    onError = () => {},
    scanIntervalMs = 250,
  } = options;

  let stream = null;
  let detector = null;
  let canvas = null;
  let ctx = null;
  let timerId = null;
  let isRunning = false;
  let isProcessingFrame = false;
  let lastScannedCode = null;
  let lastScannedTime = 0;

  async function start() {
    if (isRunning) return;

    if (!isCameraSupported()) {
      const err = new Error('La cámara no está disponible o soportada en este dispositivo.');
      onError(err);
      throw err;
    }

    let useNative = false;
    if (isBarcodeDetectorSupported()) {
      try {
        detector = new window.BarcodeDetector({ formats: ['qr_code'] });
        useNative = true;
      } catch {
        detector = null;
        useNative = false;
      }
    }

    if (!useNative) {
      if (typeof document === 'undefined' || typeof document.createElement !== 'function') {
        const err = new Error('Canvas no disponible en este entorno.');
        onError(err);
        throw err;
      }

      try {
        canvas = document.createElement('canvas');
      } catch {
        canvas = null;
      }

      if (!canvas || typeof canvas.getContext !== 'function') {
        const err = new Error('Canvas 2D context no soportado.');
        onError(err);
        throw err;
      }

      try {
        ctx = canvas.getContext('2d', { willReadFrequently: true }) || canvas.getContext('2d');
      } catch {
        ctx = null;
      }

      if (!ctx || typeof ctx.drawImage !== 'function' || typeof ctx.getImageData !== 'function') {
        const err = new Error('No se pudo inicializar el contexto 2D de canvas.');
        onError(err);
        throw err;
      }

      const jsQR = getJsQR();
      if (!jsQR) {
        const err = new Error('Librería de decodificación QR (jsQR) no disponible.');
        onError(err);
        throw err;
      }
    }

    try {
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          video: {
            facingMode: 'environment',
            width: { ideal: 1280 },
            height: { ideal: 720 },
          },
          audio: false,
        });
      } catch (camErr) {
        if (camErr && (camErr.name === 'OverconstrainedError' || camErr.name === 'ConstraintNotSatisfiedError')) {
          stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: false });
        } else {
          throw camErr;
        }
      }

      if (videoEl) {
        videoEl.srcObject = stream;
        if (typeof videoEl.setAttribute === 'function') {
          videoEl.setAttribute('playsinline', 'true');
        }
        if (typeof videoEl.play === 'function') {
          await videoEl.play().catch(() => {});
        }
      }

      isRunning = true;
      startScanLoop(useNative);
    } catch (err) {
      stop();
      onError(err);
      throw err;
    }
  }

  function handleScannedCode(rawValue) {
    if (!rawValue) return;
    const now = Date.now();

    // Deduplicate scans within ~1.5s
    if (rawValue !== lastScannedCode || now - lastScannedTime > 1500) {
      lastScannedCode = rawValue;
      lastScannedTime = now;
      onScan(rawValue);
    }
  }

  function startScanLoop(useNative) {
    if (!isRunning || timerId !== null) return;

    timerId = setInterval(async () => {
      if (!isRunning || !videoEl) return;
      if (typeof videoEl.readyState === 'number' && videoEl.readyState < 2) return; // HAVE_CURRENT_DATA
      if (isProcessingFrame) return;

      isProcessingFrame = true;
      try {
        if (useNative && detector) {
          const barcodes = await detector.detect(videoEl);
          if (barcodes && barcodes.length > 0) {
            const rawValue = (barcodes[0].rawValue || '').trim();
            handleScannedCode(rawValue);
          }
        } else if (canvas && ctx) {
          const width = videoEl.videoWidth || videoEl.width || 0;
          const height = videoEl.videoHeight || videoEl.height || 0;
          if (width <= 0 || height <= 0) return;

          if (canvas.width !== width) canvas.width = width;
          if (canvas.height !== height) canvas.height = height;

          ctx.drawImage(videoEl, 0, 0, width, height);
          const imageData = ctx.getImageData(0, 0, width, height);
          if (!imageData || !imageData.data) return;

          const jsQR = getJsQR();
          if (jsQR) {
            const decoded = decodeFrameWithFallbacks(jsQR, imageData.data, imageData.width, imageData.height, {
              onDecodeError: (err) => console.warn('jsQR decode error:', err),
            });
            if (decoded) {
              handleScannedCode(decoded);
            }
          }
        }
      } catch (err) {
        // Individual frame detect error, keep loop running
      } finally {
        isProcessingFrame = false;
      }
    }, scanIntervalMs);
  }

  function stop() {
    isRunning = false;
    isProcessingFrame = false;

    if (timerId !== null) {
      clearInterval(timerId);
      timerId = null;
    }

    if (stream) {
      if (typeof stream.getTracks === 'function') {
        stream.getTracks().forEach((track) => {
          try {
            if (typeof track.stop === 'function') track.stop();
          } catch {}
        });
      }
      stream = null;
    }

    if (videoEl) {
      try {
        if (typeof videoEl.pause === 'function') videoEl.pause();
        videoEl.srcObject = null;
      } catch {}
    }

    detector = null;
    canvas = null;
    ctx = null;
  }

  function destroy() {
    stop();
  }

  return {
    start,
    stop,
    isScanning: () => isRunning,
    destroy,
  };
}
