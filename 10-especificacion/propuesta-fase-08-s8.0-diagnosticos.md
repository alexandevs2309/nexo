# Propuesta S8.0 R3 — Diagnósticos Fase 08

> **Estado:** APROBADA POR REVISIÓN HUMANA (2026-11-22) — S8.0 spec/contrato aprobado; **implementación de Fase 08 / S8.1 NO autorizada**  
> **Fecha consolidación R3:** 2026-11-22  
> **Fecha de aprobación humana:** 2026-11-22  
> **Fuente principal:** `propuesta-fase-08-s8.0-especificacion-normativa.md`  
> **Rango candidato:** `BAS-71xx`. Verificación R3: sin colisiones conocidas por inspección R2 (`grep` sin `BAS-71/72/73/80`); revalidar al incorporar catálogo oficial.  
> **Regla:** no reutilizar BAS-70xx; no modificar catálogo oficial en esta misión. Contenido normativo sin cambios; solo estado documental actualizado.

## 1. Clasificación

| Clase | Uso |
|---|---|
| Semántico | regla de tipos/nombres/visibilidad/match/traits violada |
| Capacidad/backend | capacidad no soportada por backend/capa |
| IR/verificación | IR mal formado; normalmente interno, pero verificable por contrato |
| Reservado | código reservado para evolución; sin emisión en v1 |

## 2. Tabla normativa

| Código | Nombre | Clase | v1 | Condición de emisión | Etapa |
|---|---|---|---:|---|---|
| BAS-7101 | TipoNominalDesconocido | semántico | activo | Tipo nominal no resuelto en ámbito/imports visibles | S8.1+ |
| BAS-7102 | TipoNominalDuplicadoOSombreado | semántico | activo | Duplicado en mismo ámbito o sombreado nominal no permitido en v1 | S8.1+ |
| BAS-7103 | CampoStructDesconocido | semántico | activo | Campo inexistente en struct | S8.1 |
| BAS-7104 | CampoStructDuplicado | semántico | activo | Campo repetido en literal struct | S8.1 |
| BAS-7105 | CampoStructFaltante | semántico | activo | Falta campo obligatorio en construcción | S8.1 |
| BAS-7106 | CampoStructSobrante | semántico | activo | Campo no pertenece al struct | S8.1 |
| BAS-7107 | TipoCampoIncompatible | semántico | activo | Tipo de inicializador/asignación incompatible con campo | S8.1 |
| BAS-7108 | ConstruccionNominalInvalida | semántico | activo | Construcción nominal inválida (forma general) | S8.1+ |
| BAS-7109 | VarianteEnumDesconocida | semántico | activo | Variante inexistente en enum | S8.2 |
| BAS-7110 | PayloadVarianteIncompatible | semántico | activo | Aridad/tipo de payload incompatible | S8.2 |
| BAS-7111 | ConstruccionVarianteInvalida | semántico | activo | Forma de variante inválida; incluye payload struct-like diferido en v1 | S8.2 |
| BAS-7112 | PatronIncompatible | semántico | activo | Patrón incompatible con tipo scrutinee | S8.3 |
| BAS-7113 | MatchNoExhaustivo | semántico | activo | Cobertura incompleta de `coincidir` | S8.3 |
| BAS-7114 | PatronInalcanzable | semántico | activo | Patrón/brazo redundante o inalcanzable (error en v1) | S8.3 |
| BAS-7115 | BindingPatronInvalido | semántico | activo | Binding duplicado o inválido en patrón | S8.3 |
| BAS-7116 | UsoPatronInvalido | semántico | activo | Patrón usado fuera de contexto permitido | S8.3 |
| BAS-7117 | MetodoNoEncontrado | semántico | activo | Método inexistente para tipo estático | S8.4 |
| BAS-7118 | FirmaMetodoIncompatibleOColision | semántico | activo | Firma incompatible, colisión inherente/trait o colisión campo/método | S8.4 |
| BAS-7119 | TraitNoEncontrado | semántico | activo | Rasgo/trait no resuelto | S8.4 |
| BAS-7120 | TraitNoImplementado | semántico | activo | Tipo no implementa rasgo requerido | S8.4 |
| BAS-7121 | ImplIncompleta | semántico | activo | Falta método requerido por rasgo | S8.4 |
| BAS-7122 | ImplDuplicadaOCoherencia | semántico | activo | Impl duplicada o viola coherencia local v1 | S8.4 |
| BAS-7123 | BoundNoSatisfecho | semántico | reservado | Bounds futuros no satisfechos | Futuro |
| BAS-7124 | CapacidadNominalNoSoportadaBackend | capacidad/backend | activo | Capacidad nominal no soportada por backend/capa | S8.1+ |
| BAS-7125 | GenericoNominalNoSoportado | semántico/capacidad | activo | Uso de nominal genérico diferido en v1 | S8.1+ |
| BAS-7126 | IgualdadNominalNoSoportada | semántico | activo | `==`/`!=` sobre nominal no soportado en v1 | S8.1+ |
| BAS-7127 | ComparacionNominalIncompatible | semántico | reservado | Comparación nominal incompatible futura | Futuro |
| BAS-7128 | MiembroNoVisible | semántico | activo | Campo/método/tipo no visible en contexto | S8.1+ |
| BAS-7129 | ResolucionMetodoAmbigua | semántico | reservado | Ambigüedad multi-trait futura | Futuro |
| BAS-7130 | RecursividadNominalPorValor | semántico | activo | Ciclo de layout por valor directo/indirecto | S8.1/S8.2 |
| BAS-7131 | LvalueNominalNoSoportado | semántico | reservado | Forma de lvalue compuesta no admitida en v1 (p. ej. `id.campo[i]`, `id[i].campo`) | S8.1 |

## 3. Mensajes normativos (plantillas)

Los mensajes son estables en forma; pueden incluir contexto adicional entre corchetes.

- BAS-7101: `tipo nominal '<T>' no encontrado en el ámbito actual`
- BAS-7102: `el tipo nominal '<T>' ya está declarado o su sombreado no está permitido en v1`
- BAS-7103: `el campo '<c>' no existe en la estructura '<T>'`
- BAS-7104: `el campo '<c>' está duplicado en la construcción de '<T>'`
- BAS-7105: `falta el campo '<c>' en la construcción de '<T>'`
- BAS-7106: `el campo '<c>' no pertenece a la estructura '<T>'`
- BAS-7107: `el campo '<c>' de '<T>' espera <esperado> y recibió <obtenido>`
- BAS-7108: `construcción inválida del tipo nominal '<T>': <detalle>`
- BAS-7109: `la variante '<v>' no existe en la enumeracion '<T>'`
- BAS-7110: `la variante '<T>.<v>' espera payload <esperado> y recibió <obtenido>`
- BAS-7111: `construcción inválida de la variante '<T>.<v>': <detalle>`
- BAS-7112: `patrón incompatible con el tipo <tipo>: <detalle>`
- BAS-7113: `coincidir no exhaustivo para <tipo>: faltan <casos>`
- BAS-7114: `patrón inalcanzable en coincidir: <detalle>`
- BAS-7115: `binding inválido en patrón: <detalle>`
- BAS-7116: `uso inválido de patrón: <detalle>`
- BAS-7117: `método '<m>' no encontrado para el tipo '<T>'`
- BAS-7118: `firma o resolución de método incompatible para '<m>' en '<T>': <detalle>`
- BAS-7119: `rasgo '<R>' no encontrado`
- BAS-7120: `el tipo '<T>' no implementa el rasgo '<R>'`
- BAS-7121: `la implementación de '<R>' para '<T>' no implementa '<m>'`
- BAS-7122: `implementación duplicada o inválida por coherencia de '<R>' para '<T>': <detalle>`
- BAS-7124: `capacidad nominal '<cap>' no soportada por el backend '<b>' en capa '<capa>'`
- BAS-7125: `tipos nominales genéricos no soportados en Fase 08 v1: '<T>'`
- BAS-7126: `igualdad nominal no soportada en Fase 08 v1 para '<T>'` (aplica solo a nominales Fase 08; no altera igualdad de `Objeto` dinámico ni tipos existentes)
- BAS-7128: `miembro '<m>' de '<T>' no es visible en este contexto`
- BAS-7130: `recursividad por valor no permitida en tipo nominal '<T>': <ciclo>`
- BAS-7131 (reservado): `forma de lvalue no soportada en Fase 08 v1: <forma>`

## 4. Requisitos de pruebas por diagnóstico activo

Cada diagnóstico activo v1 debe tener:

- Prueba negativa que lo emite.
- Verificación de código BAS-71xx.
- Contexto mínimo (tipo/campo/variante/método según aplique).
- Caso TW↔VM equivalente cuando la capacidad esté en alcance TW↔VM.
- Caso de no emisión en escenario positivo equivalente.