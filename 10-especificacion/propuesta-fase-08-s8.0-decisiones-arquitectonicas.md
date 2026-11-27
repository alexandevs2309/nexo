# Propuesta S8.0 R3 — Registro de decisiones arquitectónicas Fase 08

> **Estado:** APROBADA POR REVISIÓN HUMANA (2026-11-22) — S8.0 spec/contrato aprobado; **implementación de Fase 08 / S8.1 NO autorizada**  
> **Fecha consolidación R3:** 2026-11-22  
> **Fecha de aprobación humana:** 2026-11-22  
> **Fuente principal:** `propuesta-fase-08-s8.0-especificacion-normativa.md`  
> **Regla:** No quedan alternativas A/B/C abiertas para decisiones necesarias de implementación. R3 corrige bloqueos C1–C3 sin reabrir decisiones ya resueltas. Contenido normativo sin cambios; solo estado documental actualizado.

## Decisiones

### D8-01 — Identidad nominal, ámbitos y resolución
- **Decisión R2:** Identidad por `TipoNominalId` estable + `modulo_id` + nombre canónico + clase (Struct/Enum/Trait). Compatibilidad solo por `TipoNominalId`. Espacio de nombres de tipos separado de valores; campos por struct; variantes por enum; métodos por tipo. Recolección de declaraciones por módulo antes de resolver cuerpos. Forward refs intra-módulo permitidas. Duplicado/sombreado nominal en v1 prohibido. Visibilidad por `publico` y reglas de módulo.
- **Justificación:** evita colisiones textuales/estructurales, soporta módulos, recursividad y evolución G4-ext.
- **Consecuencias:** tabla de tipos nominales, fase de declaración, IDs canónicos en HIR/MIR/runtime.
- **Compatibilidad G4:** no modifica G4; TipoNominalId es ortogonal a TipoParamId.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-02 — Representación runtime
- **Decisión R2:** Mantener `Valor::Objeto` dinámico. Introducir conceptualmente `Valor::StructNominal { tipo, campos_layout_ordenado }` y `Valor::EnumNominal { tipo, variante, payload }`. Layout físico por backend libre si preserva invariantes semánticos. Campos orden canónico de declaración; acceso por índice resuelto en compilación.
- **Justificación:** preserva Objeto dinámico, evita ambigüedad nominal, permite optimización de layout.
- **Consecuencias:** extensiones en valor/evaluador/VM/backends; debug textual estable; no JSON automático nominal.
- **Compatibilidad G4:** sin impacto.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-03 — Sintaxis canónica
- **Decisión R2:** `estructura`, `enumeracion`, `coincidir`, `rasgo`, `implementar`; receptor `yo` / `yo var`. Sin aliases en v1. Constructores `Tipo { campo: expr }`, variantes `Tipo.Variante(...)`, patrones homólogos.
- **Corrección R3 (C1):** desambiguación normativa: literal struct directo `Ident { ... }` prohibido en cabezas seguidas por bloque (`si`, `mientras`, `para cada ... en ...`, `coincidir ... { brazos }`). Usar paréntesis `( ... )`. Regla implementable en parser descendente recursivo.
- **Justificación:** coherencia lingüística y eliminación de ambigüedad parser-bloque.
- **Consecuencias:** parser debe rechazar forma ambigua con diagnóstico sintáctico claro.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-04 — Mutabilidad, copia y receptores
- **Decisión R2:** Valor-semantics con copia (`Valor::Clone`), preservando semántica de handles Arc existentes. Mutabilidad por binding/lvalue. `yo` inmutable copia; `yo var` copia mutable local sin efecto sobre llamador. Sin referencias/borrowing.
- **Corrección R3 (C2):** lvalues v1 admitidos exactamente: `id = expr`, `id.campo = expr`, `id[indice] = expr`. Rechazados en v1: proyecciones encadenadas (`id.a.b`, `id.a[i]`, `id[i].a`, `id[i][j]`) y bases no identificador. Orden: resolver base mutable; evaluar proyección una vez; evaluar RHS; validar; mutar. Atomicidad observable todo-o-nada. TW↔VM paridad obligatoria.
- **Justificación:** subconjunto coherente con runtime actual y extensible sin prometer semántica no garantizable.
- **Consecuencias:** formas encadenadas quedan diferidas; diagnóstico explícito de forma no soportada.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-05 — Igualdad
- **Decisión R2:** `==`/`!=` sobre structs/enums nominales **no soportado en v1**. Emitir BAS-7126. No igualdad estructural/textual/identidad implícita.
- **Justificación:** evita comprometer semántica antes de derivaciones/traits; comportamiento inequívoco.
- **Consecuencias:** usuarios usan comparación explícita por campos/match o helpers.
- **Compatibilidad:** sin impacto en tipos existentes.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-06 — Structs
- **Decisión R2:** Campos explícitos únicos, struct vacío permitido. Construcción exige todos los campos, sin extras/duplicados. Inicializadores evaluados en **orden de declaración** (determinista), aunque el literal se escriba en otro orden. Acceso/assign por campo visible; asignación solo en lvalue mutable.
- **Justificación:** determinismo con efectos secundarios; diagnóstico claro.
- **Consecuencias:** semántica/HIR deben reordenar/evaluar según layout declarado.
- **Compatibilidad:** extensible con defaults futuros.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-07 — Enums/ADTs
- **Decisión R2:** Variantes unitarias y payload posicional en v1. Payload struct-like diferido. Sin discriminantes de usuario. Construcción `Tipo.Variante` / `Tipo.Variante(...)`.
- **Justificación:** cubre ADT núcleo con menor complejidad de patrón/construcción.
- **Consecuencias:** BAS-7111 para forma diferida/inválida.
- **Compatibilidad:** extensible.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-08 — Pattern matching y exhaustividad
- **Decisión R2:** `coincidir` es expresión. Scrutinee evaluado una vez. Brazos en orden fuente, first-match. Patrones v1: `_`, binding, literal, variante, struct, anidados. Sin guards/or/rangos/listas/`..`. Binding duplicado prohibido. Exhaustividad obligatoria; algoritmo de cobertura por matriz/constructores. Inalcanzable = **error** BAS-7114.
- **Justificación:** seguridad, determinismo, implementabilidad sin heurísticas.
- **Consecuencias:** pass de exhaustividad/reachability; lowering a árbol de decisión.
- **Compatibilidad:** reutiliza SwitchInt en MIR.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-09 — Métodos inherentes
- **Decisión R2:** `implementar Tipo { ... }` con métodos de instancia (`yo`/`yo var`) y asociados (sin receptor). Resolución estática. No colisión campo/método. Inherente vs trait mismo nombre: error de colisión v1.
- **Justificación:** valor incremental antes de traits; evita ambigüedad.
- **Consecuencias:** tabla de métodos por TipoNominalId.
- **Compatibilidad:** base para S8.4b.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-10 — Traits estáticos mínimos
- **Decisión R2:** `rasgo` con firmas requeridas; `implementar Rasgo para Tipo` completo. Sin default methods, supertraits, associated types/constants, trait objects, dyn dispatch, bounds, impl genéricas. Coherencia local: impl en módulo del tipo o del trait. Duplicado = BAS-7122.
- **Justificación:** abstracción útil mínima, bajo riesgo.
- **Consecuencias:** resolución estática solo por tipo concreto.
- **Compatibilidad:** compatible con G4 (sin bounds).
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-11 — Compatibilidad G4
- **Decisión R2:** G4 intacto, M1–M3 intactos. Nominal concreto permitido donde ya es tipo concreto. Param opaco no obtiene capacidades nominales. Genéricos nominales diferidos; uso = BAS-7125.
- **Justificación:** preserva Fase 07 cerrada; evita reabrir monomorfización.
- **Consecuencias:** futura G4-ext requiere spec separada.
- **Compatibilidad:** total con contratos cerrados.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-12 — Recursividad y layout
- **Decisión R2:** Prohibida recursividad por valor directa/indirecta. Permitida recursividad indirecta por `lista<T>` y `arreglo<T,n>`. Detección por grafo de dependencias por valor; ciclo = BAS-7130.
- **Justificación:** garantiza tamaño/layout finito; coherente con representación actual de colecciones.
- **Consecuencias:** análisis semántico post-recolección.
- **Compatibilidad:** sin impacto en G4.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-13 — Visibilidad y módulos
- **Decisión R2:** Tipos privados por defecto, `publico` expone. Campos privados por defecto, `publico` por campo. Variantes siguen visibilidad del enum. Métodos privados por defecto, `publico` opcional. Acceso no visible = BAS-7128. Impl coherencia local tipo o trait.
- **Corrección R3 (C3):** anclado a mecanismos existentes `publico`/`exportar`/`usar`/`importar`; sin sistema paralelo. Impl no exportable por nombre; efecto visible cuando tipo y rasgo visibles y impl coherente. Duplicado/coherencia detectado en recolección semántica del programa analizado (unidad de compilación completa). Soporte nominal en módulos es requisito de implementación S8.1/S8.4, no capacidad runtime actual.
- **Justificación:** compatibilidad con arquitectura de módulos existente.
- **Consecuencias:** integración con cargador/semántica de módulos en implementación.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).

### D8-14 — Backends
- **Decisión R2:** Clasificación por capa: semántica, HIR/MIR, codegen, ejecución. TW↔VM obligatorio en subconjunto aprobado. Nativo/WASM por capacidad declarada; falta de soporte = BAS-7124, no codegen inválido.
- **Justificación:** evita bloqueo por backends parciales y evita afirmaciones falsas.
- **Consecuencias:** matriz por capacidad/capa y gates de evidencia.
- **Compatibilidad:** consistente con Fase 05/07.
- **Estado:** Resuelta en propuesta (pendiente aprobación humana).