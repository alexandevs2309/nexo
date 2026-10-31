# Consolidación de R1 y R2 — BASILISCO

## Cambios realizados

### R1 (`protocolo-registro-paquetes-r1.md`)
- Se documentó el comportamiento de `yanked` como estado administrado por el registro.
- Se definieron los campos del lock file: `registry`, `namespace`, `name`, `version`, `source`, `checksum`, `dependencies`, `verification`.
- Se distinguió integridad, autenticidad y procedencia.
- Se documentaron requisitos conceptuales pendientes para R3 (claves públicas, rotación, revocación).

### R2 (`formato-paquetes-bpk-r2.md`)
- Se aclaró que BPK es independiente del registro.
- Se documentó que `yanked` no es una propiedad del artefacto BPK.
- Se reforzó la separación entre metadatos externos y contenido intrínseco.

## Secciones afectadas
- R1: Versionado, Lock file, Seguridad (Autenticidad).
- R2: Identidad y versionado.

## Decisiones documentadas
- `yanked` es estado de registro, no del artefacto.
- Lock file distingue integridad, autenticidad y procedencia.
- BPK es independiente del registro.

## Decisiones pendientes para R3
- Distribución de claves públicas.
- Rotación y revocación.
- Relación firma-artefacto.

## Compatibilidad con el gestor local
- Se mantuvo compatibilidad conceptual con el lock file existente.
- No se modificó código fuente.

## Revisión de consistencia
- No se detectaron contradicciones reales.
- La terminología es uniforme.
- Las omisiones menores quedaron documentadas.

No se inventaron decisiones no justificadas. No se implementó R3. No se modificó código fuente. No se tocó el archivo local `q`. No se crearon ramas ni commits.