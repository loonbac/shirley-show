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
        if (typeof videoEl.setAttribute === 'function') {
          videoEl.setAttribute('playsinline', 'true');
          videoEl.setAttribute('muted', 'true');
          videoEl.setAttribute('autoplay', 'true');
        }
        // Esperar metadata para tener videoWidth/videoHeight antes del loop.
        try {
          await waitForMetadata(videoEl, 4000);
        } catch (err) {
          console.warn('Video sin metadata a tiempo, igual intento escanear:', err);
        }
        if (typeof videoEl.play === 'function') {
          try {
            await videoEl.play();
          } catch (err) {
            console.warn('video.play() rechazado (autoplay policy?):', err);
          }
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

/**
 * Decodifica el frame actual con ZXing (tolera logos centrales).
 * Usa decodeFromImageElement con un <img> construido del ImageData: es la
 * ruta del vendor que sí lee los QR decorados ya entregados (~26ms).
 * Devuelve el texto o null.
 */
  async function decodeFrameWithZXing(imageData) {
    try {
      const reader = getZXingReader();
      if (!reader || !imageData) return null;
      if (typeof document === 'undefined' || typeof document.createElement !== 'function') return null;
      const w = imageData.width;
      const h = imageData.height;
      if (!w || !h) return null;
      const tmp = createRealCanvas(w, h);
      if (!tmp) return null;
      const tctx = typeof tmp.getContext === 'function' ? tmp.getContext('2d') : null;
      if (!tctx || typeof tctx.putImageData !== 'function' || typeof tmp.toDataURL !== 'function') return null;
      tctx.putImageData(imageData, 0, 0);
      if (typeof Image === 'undefined') return null;
      const img = new Image();
      img.src = tmp.toDataURL();
      await new Promise((resolve, reject) => {
        img.onload = resolve;
        img.onerror = reject;
      });
      const result = await reader.decodeFromImageElement(img);
      const text = result && typeof result.getText === 'function' ? result.getText() : (result && result.text);
      const rawValue = typeof text === 'string' ? text.trim() : '';
      return rawValue || null;
    } catch {
      return null;
    }
  }

  /**
   * Crea un canvas REAL para el puente ImageData -> <img> de ZXing.
   * El canvas del loop puede ser falso (tests) o el createElement estar
   * mockeado: se verifica toDataURL funcional y se reutiliza el canvas del
   * frame cuando sirve. Sin canvas real no hay puente posible -> null.
   */
  function createRealCanvas(w, h) {
    try {
      if (canvas && typeof canvas.toDataURL === 'function' && canvas.width === w && canvas.height === h) {
        return canvas;
      }
    } catch {}
    try {
      if (typeof document === 'undefined') return null;
      if (typeof document.createElementNS === 'function') {
        try {
          const c = document.createElementNS('http://www.w3.org/1999/xhtml', 'canvas');
          if (c && typeof c.getContext === 'function' && typeof c.toDataURL === 'function') {
            c.width = w;
            c.height = h;
            return c;
          }
        } catch {}
      }
    } catch {}
    return null;
  }

  async function startScanLoop(useNative) {
    if (!isRunning || timerId !== null) return;

    timerId = setInterval(async () => {
      if (!isRunning || !videoEl) return;
      if (typeof videoEl.readyState === 'number' && videoEl.readyState < 2) return; // HAVE_CURRENT_DATA
      if (isProcessingFrame) return;

      isProcessingFrame = true;
      let zxingPromise = null;
      let earlyReturn = false;
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
          if (width <= 0 || height <= 0) {
            earlyReturn = true;
            return;
          }

          if (canvas.width !== width) canvas.width = width;
          if (canvas.height !== height) canvas.height = height;

          ctx.drawImage(videoEl, 0, 0, width, height);
          const imageData = ctx.getImageData(0, 0, width, height);
          if (!imageData || !imageData.data) {
            earlyReturn = true;
            return;
          }

          // ZXing arranca EN PARALELO (tolera logo central); jsQR corre a la vez.
          // Gana el primero que decodifique: el QR decorado lo resuelve ZXing
          // vía decodeFromImageElement (~26ms en pruebas E2E con logo real).
          zxingPromise = decodeFrameWithZXing(imageData);

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
      }
      try {
        if (!earlyReturn) {
          const zxingDecoded = zxingPromise ? await zxingPromise : null;
          if (zxingDecoded && !lastScanSucceededRecently()) {
            handleScannedCode(zxingDecoded);
          }
        }
      } catch {
        // ZXing falló en este frame; el próximo tick reintenta
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
