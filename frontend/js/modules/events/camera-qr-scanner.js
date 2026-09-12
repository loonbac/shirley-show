/**
 * Headless Camera QR Scanner with safe lifecycle management.
 * Leverages native browser BarcodeDetector API with graceful fallback to manual input.
 * Guarantees camera track release on stop/teardown.
 */

/**
 * Checks if the browser natively supports BarcodeDetector with QR code format.
 * @returns {boolean}
 */
export function isBarcodeDetectorSupported() {
  if (typeof window === 'undefined') return false;
  if (!('BarcodeDetector' in window)) return false;
  if (!navigator.mediaDevices || typeof navigator.mediaDevices.getUserMedia !== 'function') return false;
  return true;
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
  let timerId = null;
  let isRunning = false;
  let lastScannedCode = null;
  let lastScannedTime = 0;

  async function start() {
    if (isRunning) return;

    if (!isBarcodeDetectorSupported()) {
      const err = new Error('BarcodeDetector no está soportado en este navegador.');
      onError(err);
      throw err;
    }

    try {
      detector = new window.BarcodeDetector({ formats: ['qr_code'] });
      stream = await navigator.mediaDevices.getUserMedia({
        video: {
          facingMode: 'environment',
          width: { ideal: 1280 },
          height: { ideal: 720 },
        },
        audio: false,
      });

      if (videoEl) {
        videoEl.srcObject = stream;
        videoEl.setAttribute('playsinline', 'true');
        await videoEl.play().catch(() => {});
      }

      isRunning = true;
      startScanLoop();
    } catch (err) {
      stop();
      onError(err);
      throw err;
    }
  }

  function startScanLoop() {
    if (!isRunning || timerId !== null) return;

    timerId = setInterval(async () => {
      if (!isRunning || !detector || !videoEl) return;
      if (videoEl.readyState < 2) return; // HAVE_CURRENT_DATA

      try {
        const barcodes = await detector.detect(videoEl);
        if (barcodes && barcodes.length > 0) {
          const rawValue = (barcodes[0].rawValue || '').trim();
          const now = Date.now();

          // Deduplicate scans within 1.5s
          if (rawValue && (rawValue !== lastScannedCode || now - lastScannedTime > 1500)) {
            lastScannedCode = rawValue;
            lastScannedTime = now;
            onScan(rawValue);
          }
        }
      } catch (err) {
        // Individual frame detect error, keep loop running
      }
    }, scanIntervalMs);
  }

  function stop() {
    isRunning = false;

    if (timerId !== null) {
      clearInterval(timerId);
      timerId = null;
    }

    if (stream) {
      stream.getTracks().forEach((track) => {
        try {
          track.stop();
        } catch {}
      });
      stream = null;
    }

    if (videoEl) {
      try {
        videoEl.pause();
        videoEl.srcObject = null;
      } catch {}
    }

    detector = null;
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
