# Auditoría de coherencia R1 ↔ R2 — BASILISCO

## 1. Resumen ejecutivo

Las especificaciones R1 (protocolo remoto) y R2 (formato BPK) son **coherentes en su arquitectura general** y no presentan contradicciones críticas. R1 define la comunicación y metadatos; R2 define el contenedor interno. Ambas son complementarias.

Se identifican omisiones menores (no contradicciones) que deben resolverse antes de implementar.

## 2. Hallazgos clasificados

### CRÍTICO
Ninguno.

### ALTO
Ninguno.

### MEDIO
1. **R2 no menciona el estado `yanked`**
   - R1 define versiones retiradas mediante `yanked`.
   - R2 no lo incluye en el manifiesto BPK.
   - Consecuencia: posible inconsistencia en metadatos.
   - Corrección: añadir campo `yanked` al manifiesto BPK.

2. **R2 no define el registro de procedencia en el lock file**
   - R1 exige registro de origen.
   - R2 no detalla el lock file.
   - Consecuencia: implementación incompleta.
   - Corrección: alinear R2 con R1 en la sección de lock file.

### BAJO
3. **R2 no menciona múltiples registros**
   - R1 lo define.
   - R2 es independiente del transporte.
   - No es contradicción, solo omisión.

4. **R2 no define códigos de error**
   - R1 los normaliza.
   - R2 no los menciona.
   - Omisión aceptable.

## 3. Referencias exactas

- R1 §11 (Versionado): menciona `yanked`.
- R2 §4 (Manifiesto oficial): no incluye `yanked`.
- R1 §12 (Lock file): define procedencia.
- R2 §13 (Proceso de instalación): menciona lock file pero no su estructura.

## 4. Explicación técnica

No hay contradicciones reales. Las diferencias son de alcance: R1 es protocolo, R2 es formato. Las omisiones son menores y no impiden implementación.

## 5. Consecuencias de no corregir

- Inconsistencia en metadatos de versiones retiradas.
- Lock file incompleto en implementación.

## 6. Correcciones recomendadas

- Añadir `yanked` al manifiesto BPK.
- Alinear lock file con R1.

## 7. Tabla de compatibilidad R1/R2

| Aspecto | R1 | R2 | Compatible |
|---|---|---|---|
| Identidad | namespace/name@version | namespace/name/version | Sí |
| Artefacto | .bpk | .bpk | Sí |
| Hash | SHA-256 externo | SHA-256 interno | Sí |
| Firmas | Ed25519 | R3 | Sí |
| Dependencias | JSON | TOML | Sí |
| Lock | Procedencia | Instalación | Sí |
| Seguridad | HTTPS/TLS | ZIP seguro | Sí |

## 8. Decisiones pendientes

- Definir `yanked` en R2.
- Detallar lock file en R2.
- Especificar claves públicas por registro.

## 9. Dictamen final

**APTO CON CORRECCIONES** (menores, no bloqueantes).
Las especificaciones son coherentes y pueden implementarse tras resolver las omisiones documentadas. No se detectaron contradicciones críticas. No se realizaron cambios en los documentos. No se modificó código fuente. No se tocó el archivo local `q`. No se crearon ramas ni commits.
