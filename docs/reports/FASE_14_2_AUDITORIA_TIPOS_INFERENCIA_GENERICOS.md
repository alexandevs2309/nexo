# FASE 14.2 — Auditoría forense del sistema de tipos, inferencia y genéricos

> **Fecha:** 2026-11-27  
> **Rama:** `main` | **HEAD:** `bb1bfed` (verificado)  
> **Worktree:** sucio preexistente preservado. Sin operaciones Git destructivas, sin commits/push/merge/tags/releases/publicaciones.  
> **Alcance:** sistema de tipos, inferencia, unificación, genéricos, monomorfización, coherencia estática/runtime. No se inició Fase 14.3 ni Fase 15.

## 1. Inventario de componentes examinados

| Componente | Archivo(s) | Rol |
|---|---|---|
| Tipos semánticos | `prototipo/src/semantico.rs` (`TipoSem`, `FuncionSig`, `ParamTipoDecl`, `TipoNominalInfo`) | Representación de tipos, compatibilidad, unificación, inferencia, chequeos. |
| Anotaciones AST | `prototipo/src/parser.rs` (`TipoAnotacion`) | Tipos anotados por usuario. |
| HIR | `prototipo/src/hir.rs` | Lowering/verificación HIR, compatibilidad de retorno. |
| MIR | `prototipo/src/mir.rs` | Verificación MIR, compatibilidad de tipos. |
| Monomorfización G4 | `prototipo/src/monomorfizacion.rs` | Canonicalización, instanciación, TipoCanon. **Componente congelado**; solo adaptación mecánica por cambio de forma de `Coleccion`. |
| Runtime/VM | `prototipo/src/runtime.rs`, `prototipo/src/vm.rs` | Conversión/runtime tipo checks. |
| Backends | `prototipo/src/backend_nativo.rs`, `prototipo/src/backend_entero.rs` | Emisión de tipos/TipoConv. |
| Tests | `conformidad_tipos`, `inferencia_tipos`, `semantico`, `genericos_frontend_g2`, `genericos_semantica_g2_2`, `genericos_semantica_g3`, `genericos_monomorfizacion_g4`, `hir`, `mir`, conformidad relacionadas | Cobertura focal. |

## 2. Arquitectura real del flujo de tipos

```text
Código fuente
  -> Lexer/Parser (AST + TipoAnotacion)
  -> Análisis semántico (ContextoSemantico)
       - resolver_tipo_anotacion / TipoSem
       - inferencia local (ArregloLiteral->Coleccion, retornos C.1/C.2, etc.)
       - compatible/comparables/ordenables/tipo_comun
       - unificación G3 (sustitución sigma, TipoParamId)
       - validación V3/G4 (solicitudes)
  -> HIR (lowering + verificación tipos_compatibles_retorno)
  -> Monomorfización G4 (TipoCanon, canonicalizar_tipo, instancias)
  -> MIR (verificación tipos_compatibles_mir)
  -> Ejecución VM/runtime (conversiones, checks de asignación/contenedor)
```

## 3. Hallazgos

| ID | Severidad | Estado | Resumen |
|---|---|---|---|
| FE-14.2-01 | HIGH | **CORREGIDO** | `compatible` aceptaba `Coleccion`/`Lista` contra `Arreglo<T,N>` ignorando tamaño, permitiendo aceptación estática de asignaciones rechazadas en runtime por tamaño/contenedor incompatible. |

### FE-14.2-01 — Compatibilidad Arreglo/Coleccion/Lista ignora tamaño

- **Comportamiento esperado:** asignación a `arreglo<T,N>` debe rechazar estáticamente literales/contenedores cuyo tamaño no pueda demostrarse compatible con `N`.
- **Comportamiento observado (antes):** semántico aceptaba `var x: arreglo<entero, 3> = [1, 2]` y `var a: lista<entero>=[1,2,3]; var b: arreglo<entero,2>=a`; runtime rechazaba por incompatibilidad de tamaño.
- **Reproducción mínima:**
  ```bsk
  var x: arreglo<entero, 3> = [1, 2]
  ```
  Semántico antes: sin errores. Runtime: `asignación incompatible: se esperaba arreglo<entero, 3> y se obtuvo lista`.
- **Causa raíz:** `TipoSem::Coleccion` no portaba tamaño literal; `compatible` tenía `(Arreglo(te,_), Coleccion(to)) => compatible(te,to)` y `(Arreglo(te,_), Lista(to)) => compatible(te,to)`, ignorando tamaño. `Coleccion` tampoco distinguía longitud en unificación/compatibilidad.
- **Severidad:** HIGH (programa inválido aceptado estáticamente; garantía de tipos rota frente a ejecución real).
- **Corrección:** `TipoSem::Coleccion(Box<TipoSem>, usize)` porta tamaño literal; se actualizó construcción en inferencia de `ArregloLiteral`, `compatible`, `unificar`, `sustituir`, `tipo_comun` (sin cambio de comportamiento inesperado por `==` estructural), helpers y conversiones relacionadas. `compatible(Arreglo, Coleccion)` exige mismo tamaño; `compatible(Arreglo, Lista)` rechaza (tamaño estáticamente desconocido); `compatible(Lista, Arreglo)` permanece permitido (lista acepta arreglo de cualquier tamaño). Se adaptó mecánicamente TipoConv/runtime, HIR, MIR, backend_nativo/backend_entero/vm y TipoCanon/monomorfización G4 al nuevo shape.
- **Regresión:** `prototipo/tests/conformidad_tipos.rs` añade `literal_coleccion_tamano_incompatible_con_arreglo_es_error`, `literal_coleccion_tamano_compatible_con_arreglo_aceptado`, `literal_coleccion_tamano_incompatible_rechazado_por_semantico`.
- **Resultado posterior:** semántico rechaza tamaño incompatible; casos válidos siguen aceptados.

## 4. Reproducciones mínimas

```bsk
# FE-14.2-01 / caso A
var x: arreglo<entero, 3> = [1, 2]  # debe ser error estático
```

```bsk
# FE-14.2-01 / caso B
var a: lista<entero> = [1, 2, 3]
var b: arreglo<entero, 2> = a  # tamaño desconocido estáticamente: rechazado
```

```bsk
# válido
var x: arreglo<entero, 2> = [1, 2]
var y: lista<entero> = [1, 2]
```

## 5. Causa raíz

Pérdida de información de tamaño en `Coleccion` y reglas de compatibilidad asimétricas/incompletas para contenedores: `Arreglo` requería tamaño en comparaciones arreglo-arreglo, pero no cuando el origen era `Coleccion`/`Lista`.

## 6. Archivos modificados

- `prototipo/src/semantico.rs` — `TipoSem::Coleccion` con tamaño; inferencia literal; `compatible`/`unificar`/sustitución/patrones.
- `prototipo/src/runtime.rs` — `TipoConv::Coleccion` con tamaño (comportamiento runtime conservador).
- `prototipo/src/backend_nativo.rs` — emisión TipoConv Coleccion.
- `prototipo/src/backend_entero.rs`, `prototipo/src/vm.rs` — patrones Coleccion.
- `prototipo/src/hir.rs` — compatibilidad retorno y test HIR adaptado.
- `prototipo/src/mir.rs` — compatibilidad MIR/patrones.
- `prototipo/src/monomorfizacion.rs` — adaptación mecánica TipoCanon/Coleccion y construcciones (componente congelado, sin cambio lógico G4).
- `prototipo/tests/conformidad_tipos.rs` — regresiones FE-14.2-01.

## 7. Correcciones realizadas

- Defecto FE-14.2-01 corregido con ajuste de representación de tipo y reglas de compatibilidad.
- Adaptación mecánica en G4/IR/backends/runtime para compilar y preservar semántica.

## 8. Pruebas de regresión

- `conformidad_tipos::literal_coleccion_tamano_incompatible_con_arreglo_es_error`
- `conformidad_tipos::literal_coleccion_tamano_compatible_con_arreglo_aceptado`
- `conformidad_tipos::literal_coleccion_tamano_incompatible_rechazado_por_semantico`

## 9. Resultados exactos de comandos

Ejecutados en `prototipo/` salvo indicado:

```text
$ cargo test --test conformidad_tipos --test inferencia_tipos --test genericos_frontend_g2 --test genericos_semantica_g2_2 --test genericos_semantica_g3 --test genericos_monomorfizacion_g4 --test hir --test mir
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --test semantico
test result: ok. 109 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --test conformidad_arreglos_listas --test conformidad_funciones --test conformidad_semantica --test conformidad_conversiones --test conformidad_control
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --check
(no output, exit 0)

$ git diff --check
OK (exit 0)
```

No se ejecutó `cargo test --workspace` completo; no se declara aprobación global del workspace.

## 10. Componentes congelados examinados / reabiertos

| Componente | Examinado | Reapertura lógica | Acción |
|---|---|---|---|
| G4 monomorfización (`monomorfizacion.rs`) | Sí | No | Adaptación mecánica por cambio de shape `Coleccion`; pruebas G4 33/33 PASS. |
| HIR020/E1010 | No reabiertos | No | Sin evidencia de regresión. |

## 11. Limitaciones conocidas

- Endurecimiento `Lista -> Arreglo<N>`: semántico ahora rechaza estáticamente porque el tamaño de lista es desconocido; el runtime podía aceptar si la longitud coincidía en tiempo de ejecución. Se documenta como mejora de solidez, no como ampliación de feature.
- No se ejecutó workspace completo ni todas las pruebas diferenciales VM/backends en esta misión.
- Auditoría no exhaustiva de todos los rincones de inferencia G3/G4; se priorizaron áreas de mayor riesgo y coherencia estática/runtime.

## 12. Riesgos residuales

- **Medio:** pueden existir otros puntos donde `Desconocido`/compatibilidad permisiva oculten errores estáticos.
- **Medio:** el cambio de forma de `Coleccion` es amplio; focal suites pasan, pero no hay ejecución de workspace completo.
- **Bajo:** código existente que asignaba `lista` a `arreglo<N>` contando con longitud runtime ahora será rechazado estáticamente.

## 13. Dictamen final

**INCOMPLETE — FINDINGS DOCUMENTED**

Se examinó el flujo real de tipos/inferencia/genéricos/IR y se corrigió FE-14.2-01 (HIGH) con regresiones y suites focales verdes. Dada la amplitud de la misión y la no ejecución del workspace completo, no se declara certificación global del sistema de tipos. No se inició Fase 14.3 ni Fase 15.
