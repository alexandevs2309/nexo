# Matriz de trazabilidad de requisitos

## Convención
Los identificadores `REQ-*` son estables. Cada requisito debe enlazarse a una regla normativa, a una o más pruebas y a evidencia de implementación. Las rutas de prueba son marcadores a completar con nombres reales de tests en el repositorio.

| ID | Requisito | Fuente normativa | Pruebas mínimas | Estado inicial |
|---|---|---|---|---|
| REQ-001 | El lexer debe rechazar tokens mal formados con ubicación | Gramática/errores | cadenas sin cerrar, caracteres inválidos, entradas Unicode | Por verificar |
| REQ-002 | La precedencia de operadores debe ser determinista | Especificación Core §5 | tabla de precedencia, agrupación con paréntesis | Por verificar |
| REQ-003 | División entera trunca hacia cero | Core §5 / ADR-0005 | positivos, negativos, divisores ±1 | Referencia documentada; verificar CI |
| REQ-004 | División por cero produce error controlado | Core §5 / ADR-0005 | entero, decimal, real | Referencia documentada; verificar CI |
| REQ-005 | Resto conserva el signo del dividendo | Core §5 / ADR-0005 | combinaciones de signos | Referencia documentada; verificar CI |
| REQ-006 | Lógica usa cortocircuito | Core §6 / ADR-0007 | RHS con error, RHS con efecto al futuro | Referencia documentada; verificar CI |
| REQ-007 | Condiciones requieren `lógico` | Core §6 | condición entera/decimal/real | Referencia documentada; verificar CI |
| REQ-008 | `decimal` y `real` no se comparan directamente | Core §6 / ADR-0007 | seis comparadores, ambos órdenes | Referencia documentada; verificar CI |
| REQ-009 | Identificadores no declarados producen error | Variables/errores | lectura y asignación desconocidas | Por verificar |
| REQ-010 | Los bloques tienen ámbito documentado | Variables/control | sombreado, escape de variables, duplicados | Por verificar |
| REQ-011 | `si/sino/mientras` siguen gramática definida | Core §7 | ramas, anidación, cero iteraciones, una iteración | Referencia documentada; verificar CI |
| REQ-012 | No se anuncia función hasta especificar retorno | GAP-005 | llamadas, aridad, recursión, retorno inválido | Pendiente |
| REQ-013 | Los errores no filtran pánicos internos al usuario | Errores | fuzzing y entradas malformadas | Por verificar |
| REQ-014 | Builds y pruebas son reproducibles | Reproducibilidad | CI limpia, dependencias fijadas, artefactos | Pendiente de evidencia |
| REQ-015 | Toda capacidad de plataforma tiene matriz de soporte | Estrategia de plataformas | smoke tests por OS/arquitectura | Pendiente |
| REQ-016 | Dependencias se verifican antes de ejecutarse | Paquetes/seguridad | checksum, firma, permisos, paquete manipulado | Futuro |
| REQ-017 | Los cambios incompatibles requieren migración | Compatibilidad | pruebas API y guías de migración | Propuesto |
| REQ-018 | La documentación coincide con la implementación | Gobernanza | ejemplos ejecutados en CI | Pendiente |
| REQ-019 | Diagnósticos tienen código estable y ubicación | Errores | snapshots de códigos/spans | Propuesto |
| REQ-020 | La suite normativa identifica la versión probada | Release gates | reporte con versión/commit/plataforma | Propuesto |

## Cómo mantenerla
- Cada PR que cambie el lenguaje debe actualizar esta tabla.
- Una prueba debe tener un identificador o nombre estable y ejecutarse en CI.
- Los estados posibles son: `pendiente`, `especificado`, `implementado`, `verificado`, `bloqueado`.
- No marcar `verificado` sin salida de CI del commit exacto.
