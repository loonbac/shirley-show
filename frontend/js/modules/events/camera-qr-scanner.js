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
 * Resuelve el lector ZXing (BrowserQRCodeReader) desde el global si el
 * vendor zxing.min.js está cargado. ZXing sí decodifica los QR decorados
 * con logo central que jsQR no puede leer.
 * @returns {any|null} instancia de BrowserQRCodeReader o null
 */
export function getZXingReader() {
  try {
    const g = typeof globalThis !== 'undefined' ? globalThis : null;
    const ZX = (g && g.ZXing) || (typeof window !== 'undefined' && window.ZXing) || null;
    if (ZX && typeof ZX.BrowserQRCodeReader === 'function') {
      return new ZX.BrowserQRCodeReader();
    }
  } catch {}
  return null;
}

/**
 * Decodifica ImageData con ZXing de bajo nivel (sin DOM).
 * Reservado para diagnóstico/tests; el loop usa decodeFrameWithZXing
 * (decodeFromImageElement), que es la ruta que lee los QR ya entregados.
 * @param {any} ZX namespace window.ZXing
 * @param {{ data: Uint8ClampedArray, width: number, height: number }} imageData
 * @returns {string|null} payload o null
 */
export function decodeWithZXingLowLevel(ZX, imageData) {
  try {
    if (!ZX || !imageData) return null;
    const w = imageData.width;
    const h = imageData.height;
    if (!w || !h || !imageData.data) return null;
    const LumCls = ZX.HTMLCanvasElementLuminanceSource;
    const Hybrid = ZX.HybridBinarizer;
    const Bitmap = ZX.BinaryBitmap;
    const Reader = ZX.QRCodeReader;
    if (typeof LumCls !== 'function' || typeof Hybrid !== 'function'
      || typeof Bitmap !== 'function' || typeof Reader !== 'function') return null;
    // Fuente de luminancia mínima sobre el buffer RGBA (sin canvas).
    const source = {
      getWidth: () => w,
      getHeight: () => h,
      getRow: (y, row) => {
        const out = row && row.length >= w ? row : new Uint8ClampedArray(w);
        const off = y * w * 4;
        for (let x = 0; x < w; x++) {
          const i = off + x * 4;
          out[x] = Math.round((imageData.data[i] + imageData.data[i + 1] + imageData.data[i + 2]) / 3);
        }
        return out;
      },
      getMatrix: () => {
        const m = new Uint8ClampedArray(w * h);
        for (let i = 0; i < w * h; i++) {
          m[i] = Math.round((imageData.data[i * 4] + imageData.data[i * 4 + 1] + imageData.data[i * 4 + 2]) / 3);
        }
        return m;
      },
    };
    // Heredar el prototype real para pasar los chequeos internos si existen.
    try {
      Object.setPrototypeOf(source, LumCls.prototype);
    } catch {}
    const bitmap = new Bitmap(new Hybrid(source));
    const result = new Reader().decode(bitmap);
    const text = result && typeof result.getText === 'function' ? result.getText() : (result && result.text);
    const rawValue = typeof text === 'string' ? text.trim() : '';
    return rawValue || null;
  } catch {
    return null;
  }
}

/**
 * Resuelve el namespace ZXing global (window.ZXing) si está cargado.
 * Reservado para diagnóstico; el loop usa decodeFrameWithZXing.
 * @returns {any|null}
 */
/**
 * Decodifica el canvas actual con ZXing directamente de forma síncrona (~15-20ms).
 * Utiliza HTMLCanvasElementLuminanceSource y QRCodeReader sin intermediarios de datos ni imágenes.
 * @param {HTMLCanvasElement} canvasEl
 * @returns {string|null}
 */
export function decodeCanvasWithZXing(canvasEl) {
  try {
    const ZX = getZXingNamespace();
    if (!ZX || !canvasEl) return null;
    const LumCls = ZX.HTMLCanvasElementLuminanceSource;
    const Hybrid = ZX.HybridBinarizer;
    const Bitmap = ZX.BinaryBitmap;
    const Reader = ZX.QRCodeReader;
    if (typeof LumCls !== 'function' || typeof Hybrid !== 'function'
      || typeof Bitmap !== 'function' || typeof Reader !== 'function') return null;
    const source = new LumCls(canvasEl);
    const bitmap = new Bitmap(new Hybrid(source));
    const result = new Reader().decode(bitmap);
    const text = result && (typeof result.getText === 'function' ? result.getText() : result.text);
    return (typeof text === 'string' && text.trim()) ? text.trim() : null;
  } catch {
    return null;
  }
}

/**
 * Decodifica el recorte central del canvas con ZXing (cuando el usuario centra el QR en la guía).
 * @param {HTMLCanvasElement} canvasEl
 * @param {number} width
 * @param {number} height
 * @param {HTMLCanvasElement|null} [cropCanvas]
 * @returns {string|null}
 */
export function decodeCenterCropWithZXing(canvasEl, width, height, cropCanvas = null) {
  try {
    if (width < 160 || height < 160) return null;
    const cw = Math.floor(width * 0.65);
    const ch = Math.floor(height * 0.65);
    const ox = Math.floor((width - cw) / 2);
    const oy = Math.floor((height - ch) / 2);
    let c = cropCanvas;
    if (!c && typeof document !== 'undefined' && typeof document.createElement === 'function') {
      c = document.createElement('canvas');
    }
    if (!c) return null;
    if (c.width !== cw) c.width = cw;
    if (c.height !== ch) c.height = ch;
    const cctx = typeof c.getContext === 'function' ? c.getContext('2d') : null;
    if (!cctx || typeof cctx.drawImage !== 'function') return null;
    cctx.drawImage(canvasEl, ox, oy, cw, ch, 0, 0, cw, ch);
    return decodeCanvasWithZXing(c);
  } catch {
    return null;
  }
}

export function getZXingNamespace() {
  try {
    const g = typeof globalThis !== 'undefined' ? globalThis : null;
    const ZX = (g && g.ZXing) || (typeof window !== 'undefined' && window.ZXing) || null;
    if (ZX && typeof ZX.QRCodeReader === 'function') return ZX;
  } catch {}
  return null;
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
export function waitForVideoMetadata(video, timeoutMs = 4000) {
  return new Promise((resolve, reject) => {
    if (!video) return reject(new Error('sin elemento de video'));
    if (typeof video.videoWidth === 'number' && video.videoWidth > 0) return resolve();
    if (typeof video.readyState === 'number' && video.readyState >= 1) return resolve();
    let done = false;
    const timer = setTimeout(() => {
      if (!done) {
        done = true;
        cleanup();
        reject(new Error('timeout esperando metadata del video'));
      }
    }, timeoutMs);
    function onMeta() {
      if (!done) {
        done = true;
        clearTimeout(timer);
        cleanup();
        resolve();
      }
    }
    function cleanup() {
      try {
        if (typeof video.removeEventListener === 'function') {
          video.removeEventListener('loadedmetadata', onMeta);
        }
      } catch {}
    }
    try {
      if (typeof video.addEventListener === 'function') {
        video.addEventListener('loadedmetadata', onMeta, { once: true });
      } else {
        clearTimeout(timer);
        resolve();
      }
    } catch {
      clearTimeout(timer);
      resolve();
    }
  });
}

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
 *   waitForMetadata?: (video: HTMLVideoElement, timeoutMs?: number) => Promise<void>,
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
    waitForMetadata = waitForVideoMetadata,
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
  let lastScanAt = 0;

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
      // Canvas real verificable: debe poder entregar contexto 2D funcional.
      // (En tests con document.createElement mockeado, el canvas falso se
      // detecta aquí y se pide el real vía document.createElementNS si existe.)
      if (canvas && typeof canvas.getContext === 'function') {
        try {
          const probe = canvas.getContext('2d');
          if (!probe || typeof probe.getImageData !== 'function') {
            canvas = null;
          }
        } catch {
          canvas = null;
        }
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
      const zxingReader = getZXingReader();
      if (!jsQR && !zxingReader) {
        const err = new Error('Librería de decodificación QR no disponible.');
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
        videoEl.muted = true;
        if ('playsInline' in videoEl) {
          videoEl.playsInline = true;
        }
        if (typeof videoEl.setAttribute === 'function') {
          videoEl.setAttribute('playsinline', 'true');
          videoEl.setAttribute('muted', 'true');
          videoEl.setAttribute('autoplay', 'true');
        }
        if (typeof videoEl.play === 'function') {
          try {
            const p = videoEl.play();
            if (p && typeof p.catch === 'function') {
              p.catch((err) => console.warn('video.play() diferido:', err));
            }
          } catch (err) {
            console.warn('video.play() rechazado:', err);
          }
        }
        try {
          await waitForMetadata(videoEl, 3000);
        } catch (err) {
          console.warn('Video sin metadata a tiempo, igual intento escanear:', err);
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
    lastScanAt = now;

    // Deduplicate scans within ~1.5s
    if (rawValue !== lastScannedCode || now - lastScannedTime > 1500) {
      lastScannedCode = rawValue;
      lastScannedTime = now;
      onScan(rawValue);
    }
  }

  function lastScanSucceededRecently() {
    return lastScanAt > 0 && (Date.now() - lastScanAt) < 1500;
  }

  let cropCanvas = null;

  async function startScanLoop(useNative) {
    if (!isRunning || timerId !== null) return;

    timerId = setInterval(async () => {
      if (!isRunning || !videoEl) return;
      if (typeof videoEl.readyState === 'number' && videoEl.readyState < 2) return;
      if (isProcessingFrame) return;

      isProcessingFrame = true;
      try {
        let code = null;

        // 1. BarcodeDetector nativo primero si está activo
        if (useNative && detector) {
          try {
            const barcodes = await detector.detect(videoEl);
            if (barcodes && barcodes.length > 0) {
              code = (barcodes[0].rawValue || '').trim();
            }
          } catch {}
        }

        // 2. Si el nativo no detectó o no está activo, usar pipeline de canvas (ZXing + jsQR)
        if (!code && canvas && ctx) {
          const width = videoEl.videoWidth || videoEl.width || 0;
          const height = videoEl.videoHeight || videoEl.height || 0;
          if (width > 0 && height > 0) {
            if (canvas.width !== width) canvas.width = width;
            if (canvas.height !== height) canvas.height = height;

            ctx.drawImage(videoEl, 0, 0, width, height);

            // A) ZXing directo sobre el canvas completo (~15-20ms)
            code = decodeCanvasWithZXing(canvas);

            // B) ZXing sobre el recorte central (visor)
            if (!code) {
              code = decodeCenterCropWithZXing(canvas, width, height, cropCanvas);
            }

            // C) Fallback jsQR sobre el buffer RGBA
            if (!code) {
              const jsQR = getJsQR();
              if (jsQR) {
                const imageData = ctx.getImageData(0, 0, width, height);
                if (imageData && imageData.data) {
                  code = decodeFrameWithFallbacks(jsQR, imageData.data, width, height, {
                    onDecodeError: (err) => console.warn('jsQR decode error:', err),
                  });
                }
              }
            }
          }
        }

        if (code) {
          handleScannedCode(code);
        }
      } catch (err) {
        // Error de frame individual, el loop continúa
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
    cropCanvas = null;
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
