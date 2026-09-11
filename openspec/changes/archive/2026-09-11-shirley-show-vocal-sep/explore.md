# SDD Exploration: `shirley-show-vocal-sep`

**Change**: `shirley-show-vocal-sep`
**Date**: 2026-09-11
**Status**: Completed (full version in Engram `sdd/shirley-show-vocal-sep/explore`, obs. 334)

## Brief
"Sepárame esta pista (link YT)": Ressy descarga, separa voces por GPU, avisa listo,
pregunta destino. Dos pipelines fijos, sin Leap/Demucs/cascade/min-max.

## Pipelines
1. COROS: gabox + becruily + anvuew sobre original → karaoke stems → AVG_WAVE 1:1:1 float.
2. INSTRUMENTAL: solo PolarFormer (model_bs_polarformer_float16.ckpt+yaml — AUSENTE hoy → degradar amable).

## Facts
- CLI karaoke-separator (gabox-invertido|frazer-becruily|anvuew|polarformer), 2 stems, VRAM liberada al salir.
- Binario configurable VOCAL_SEP_BIN. Output template NAME_MDOEL_STEM; aislar por temp_dir y excluir stem vocal (?i)vocal.
- GPU única → Semaphore(1). Timeout HTTP debe subir (600s). Límite 10min vía ffprobe.
- Salida final MP3 320K vía ffmpeg (regla de producto).

## Design shape
VocalSeparationMode domain; avg_wave con hound; VocalSeparator port + ProcessVocalSeparator + VocalProcessGuard; SeparateVocals use case + staging/registry TTL; tool separate_vocals + prompt (preguntar modo y destino); typing "separando voces…" sin SSE.

## Slices
1. Dominio + avg_wave (~180). 2. Infra + guard + mutex (~280).
3. Use case + registry (~220). 4. Tool + prompt + frontend (~190).

## Key Learnings
1. Subprocesos liberan VRAM al salir; mutex evita OOM.
2. AVG_WAVE float interno con hound, sin ensemble externo.
3. Dos turnos: download a staging mientras se aclara modo/destino.
