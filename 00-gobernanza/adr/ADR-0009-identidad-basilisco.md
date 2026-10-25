# ADR-0009 — Identidad BASILISCO y retirada progresiva de Nexo

- Estado: **Aprobado**.
- Fecha: 2026-10-22.
- Responsable: proyecto BASILISCO.
- Revisión: al publicar la primera versión estable o si cambia la estrategia de compatibilidad.

## 1. Contexto

Nexo fue el nombre de trabajo del lenguaje. La auditoría de migración confirmó que:

- en el momento de la auditoría, el repositorio no contenía archivos `.nx` (el fixture de rechazo añadido después no representa compatibilidad);
- los únicos archivos de ejemplo con extensión propia eran dos `.nexo`;
- no existía un CLI que ejecutara archivos: `main.rs` era un demo hardcodeado;
- el paquete Rust se llamaba `nexo` y no había publicación ni workflow de release;
- el comando `bsk` ya es usado por BrowserSkill de Tencent;
- la extensión `.bsk` ya está asociada a Nero SecurDisc y Bryce Skies;
- existe un BASIC italiano llamado BAS.I.LI.CO., aunque no con la grafía exacta BASILISCO.

La decisión del proyecto es que **Nexo quede como identidad temporal/histórica** y que BASILISCO sea la identidad objetivo.

## 2. Decisión

1. **Nombre:** BASILISCO.
2. **Paquete Rust:** `basilisco`.
3. **Binario principal:** `basilisco`.
4. **Alias corto:** `bsk`, con detección de conflicto y mensajes inequívocos.
5. **Extensión canónica:** `.bsk`.
6. **Extensión legada:** `.nexo`.
7. **`.nx`:** no se acepta como extensión canónica ni como legado mientras no exista evidencia de archivos externos que justifiquen compatibilidad.
8. **Documentación:** los documentos nuevos usan BASILISCO. Los documentos históricos pueden conservar Nexo, marcados como históricos.
9. **Repositorio remoto y publicación:** no se renombran ni publican en esta fase; requieren autorización y verificación de distribución.

## 3. Consecuencias

- Los imports `use nexo` pasan a `use basilisco`.
- Los tests y ejemplos se actualizan, conservando `.nexo` para compatibilidad.
- El crate `nexo` de crates.io ya está ocupado; no se reutiliza.
- El comando `bsk` colisiona operacionalmente con BrowserSkill; el alias debe documentarse y detectar conflictos.
- `.bsk` no es exclusiva; se documenta el conflicto con Nero/Bryce y se prioriza `basilisco run`.
- La licencia se aprobó después mediante ADR-0016. Los metadatos restantes y la publicación siguen pendientes antes de distribuir.

## 4. Criterios de aceptación

- `cargo test` pasa con la suite existente más los tests de CLI.
- `basilisco --version` y `bsk --version` identifican BASILISCO.
- `basilisco run archivo.bsk` y `basilisco run archivo.nexo` funcionan.
- `.nx` se rechaza con un mensaje explícito.
- `git grep -i nexo` en código solo aparece en documentación histórica o de migración.
- No se publica ni se renombra el repositorio remoto sin autorización.

## 5. Alternativas descartadas

- **Mantener Nexo como identidad principal:** contradice la decisión del proyecto y mantiene el conflicto de nombre.
- **Usar `.nx` como extensión canónica:** no hay archivos `.nx` en el repositorio, tiene colisiones propias y conserva la marca temporal.
- **Usar solo `bsk` como comando:** aumenta el riesgo de colisión con BrowserSkill; se prefiere `basilisco` como principal y `bsk` como alias.
- **Renombrar el repositorio antes de tener CLI:** no resuelve el problema funcional y añade riesgo de enlaces y distribución.

## 6. Reversión

La decisión es reversible en código: se puede restaurar el paquete `nexo`, el binario `nexo` y la documentación histórica. No se debe reescribir el historial. Antes de aplicar cambios masivos se debe crear una etiqueta de baseline y trabajar en una rama de migración.
