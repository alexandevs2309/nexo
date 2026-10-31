# Cierre técnico R1 y R2 — BASILISCO

## Hallazgos resueltos

- Se aclaró que las rutas del manifiesto BPK son relativas a `payload/`.
- Se documentó la representación de dependencias con registro explícito.
- Se definieron los estados de verificación del lock file.
- Se aclaró la separación entre integridad, autenticidad y procedencia.
- Se documentó que Ed25519 es el algoritmo previsto, con R3 pendiente.

## Documentos y secciones modificadas

- `protocolo-registro-paquetes-r1.md`: Lock file, Versionado, Seguridad.
- `formato-paquetes-bpk-r2.md`: Identidad y versionado, Manifiesto oficial.

## Decisiones técnicas finales

- `yanked` es estado de registro, no del artefacto.
- Lock file distingue integridad, autenticidad y procedencia.
- BPK es independiente del registro.
- Rutas del manifiesto relativas a `payload/`.
- Dependencias pueden declarar registro explícito.

## Compatibilidad con el gestor local

- Se mantuvo compatibilidad conceptual con el lock file existente.
- No se modificó código fuente.

## Pendientes reservados para R3

- Distribución de claves públicas.
- Rotación y revocación.
- Relación firma-artefacto.

## Resultado de la revisión

- No se detectaron contradicciones reales.
- La terminología es uniforme.
- Las omisiones menores quedaron documentadas.

## Dictamen

**CERRADAS** — R1 y R2 quedan coherentes y preparadas para implementación futura. No se realizaron cambios de código. No se tocó el archivo local `q`. No se crearon ramas ni commits.