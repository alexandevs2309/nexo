# Índice de decisiones de arquitectura (ADR)

**Estado:** índice de referencia. La fuente de cada decisión es el ADR correspondiente en `adr/`.  
**Última revisión:** 2026-10-25.

| ADR | Título | Estado | Fecha | Archivo |
|---|---|---|---|---|
| ADR-0001 | Alcance de Nexo: lenguaje multipropósito | Aprobado | 2026-10-20 | `adr/ADR-0001-alcance-multiproposito.md` |
| ADR-0002 | Lenguaje anfitrión de la primera implementación | Aprobado | 2026-10-20 | `adr/ADR-0002-lenguaje-anfitrión.md` |
| ADR-0003 | Sintaxis léxica mínima del prototipo | Aprobado | 2026-10-20 | `adr/ADR-0003-sintaxis-lexica-minima.md` |
| ADR-0004 | Precedencia y asociatividad de operadores | Aprobado | 2026-10-20 | `adr/ADR-0004-precedencia-operadores.md` |
| ADR-0005 | Sistema numérico del núcleo | Aprobado | 2026-10-20 | `adr/ADR-0005-sistema-numerico.md` |
| ADR-0005 (anexos) | Análisis D1–D6, D2–D5 y D3–D4 | Propuesta / material de trabajo; no cierran por sí solos el sistema numérico | 2026-10-20 | `adr/ADR-0005-anexo-d1-d6.md`, `adr/ADR-0005-anexo-d2-d5.md`, `adr/ADR-0005-anexo-d3-d4.md` |
| ADR-0006 | Declaraciones, variables y ámbitos iniciales | Aprobado | 2026-10-21 | `adr/ADR-0006-declaraciones-y-variables.md` |
| ADR-0007 | Bloques, condiciones, ciclos y tipos lógicos | Aprobado | 2026-10-21 | `adr/ADR-0007-estructuras-de-control-y-logicos.md` |
| ADR-0008 | Funciones, retorno y llamadas | Aprobado e implementado | 2026-10-21 | `adr/ADR-0008-funciones-y-retorno.md` |
| ADR-0009 | Identidad BASILISCO y retirada progresiva de Nexo | Aprobado | 2026-10-22 | `adr/ADR-0009-identidad-basilisco.md` |
| ADR-0010 | CLI y ejecución de archivos | Aprobado e implementado | 2026-10-22 | `adr/ADR-0010-cli-y-extension.md` |
| ADR-0011 | Sistema de tipos gradual, parámetros tipados, retorno anotado y valor nulo | Aprobado | 2026-10-24 | `adr/ADR-0011-sistema-de-tipos-gradual.md` |
| ADR-0012 | Iteración: `para cada`, rangos, `romper` y `continuar` | Aprobado | 2026-10-24 | `adr/ADR-0012-iteracion-para-cada-romper-continuar.md` |
| ADR-0013 | E/S básica: entrada estándar, operaciones de texto y archivos | Aprobado | 2026-10-24 | `adr/ADR-0013-entrada-salida-basica-texto-archivos.md` |
| ADR-0014 | Manejo de errores: `intentar`/`rescatar` | Aprobado | 2026-10-24 | `adr/ADR-0014-manejo-de-errores-intentar-rescatar.md` |
| ADR-0015 | Aprobación del perfil de conformidad BASILISCO v0.4 | Aprobado | 2026-10-25 | `adr/ADR-0015-aprobacion-perfil-v0.4.md` |
| ADR-0016 | Licencia de publicación de BASILISCO: Apache-2.0 | Aprobado para v0.4, sujeto a auditoría de procedencia | 2026-10-25 | `adr/ADR-0016-licencia-apache-2.0.md` |

## Cómo interpretar el estado
- **Aprobado** significa que la decisión fue aceptada explícitamente; no significa que esté implementada.
- **Propuesto** significa que la decisión está preparada pero no aprobada; no debe presentarse como contrato vigente.
- La implementación se verifica en el código y en `07-validacion/matriz-de-trazabilidad.md`, no en este índice.
- Los anexos de ADR-0005 se conservan como material de análisis. El ADR-0005 principal declara la aprobación y las decisiones consolidadas; si un anexo contradice al ADR principal, se abre una incidencia de coherencia conforme a `10-especificacion/estado-normativo.md`.

## Nota de incorporación
Los ADR se copiaron desde el repositorio `alexandevs2309/nexo` en la revisión de coherencia 0.3.1 para que la base documental sea autocontenida. No se alteró su contenido sustantivo; en los anexos de ADR-0005 se añadió una nota de estado para evitar que su redacción previa contradiga el ADR-0005 consolidado y aprobado.
