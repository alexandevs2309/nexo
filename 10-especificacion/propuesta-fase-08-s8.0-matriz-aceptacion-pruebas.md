# Propuesta S8.0 R3 — Matriz de aceptación y pruebas Fase 08

> **Estado:** APROBADA POR REVISIÓN HUMANA (2026-11-22) — S8.0 spec/contrato aprobado; **implementación de Fase 08 / S8.1 NO autorizada**  
> **Fecha consolidación R3:** 2026-11-22  
> **Fecha de aprobación humana:** 2026-11-22  
> **Fuente principal:** `propuesta-fase-08-s8.0-especificacion-normativa.md`  
> **Nota:** Casos de diseño para implementación futura. No ejecutar/crear tests en esta misión. Contenido normativo sin cambios; solo estado documental actualizado.

## 1. Criterios globales

| ID | Criterio | Verificación | Evidencia |
|---|---|---|---|
| CA08-G1 | Spec S8.0 aprobada antes de S8.1 | Revisión humana | aprobación registrada |
| CA08-G2 | TW↔VM paridad en subconjunto aprobado | diferenciales | suite + reporte |
| CA08-G3 | Nativo/WASM capacidad declarada | pruebas/rechazos BAS-7124 | matriz + logs |
| CA08-G4 | No regresión Fase 07/workspace | suites existentes | conteos/exit 0 |
| CA08-G5 | Diagnósticos BAS-71xx activos estables | pruebas negativas | código+contexto |
| CA08-G6 | Integridad HIR/MIR | verificadores + IR inválida | tests IR |
| CA08-G7 | G4/M1-M3 intactos | revisión + suites G4 | sin diff contratos + verde |
| CA08-G8 | Sin cambios funcionales en S8.0 | git diff | solo docs propuestas |

## 1A. Correcciones bloqueantes R3 (C1–C3)

### C1 — Desambiguación sintáctica

| Tipo | Caso | Esperado |
|---|---|---|
| Positivo | `var p: Punto = Punto { x: 1, y: 2 }` | Construcción válida. |
| Positivo | `si (Punto { x: 1, y: 2 }).x > 0 { mostrar("ok") }` | Paréntesis desambigua; válido. |
| Positivo | `coincidir (Punto { x: 1, y: 2 }) { Punto { x: v, y: _ } => v }` | Scrutinee parentizado válido. |
| Positivo | `para cada x en (Caja { xs: [1,2] }).xs { mostrar(x) }` | Iterable parentizado válido. |
| Negativo | `si Punto { x: 1, y: 2 }.x > 0 { ... }` | Error sintáctico de desambiguación (literal struct directo en cabeza). |
| Negativo | `mientras Punto { ok: verdadero }.ok { ... }` | Error sintáctico de desambiguación. |
| Negativo | `coincidir Punto { x: 1, y: 2 } { ... }` | Error sintáctico de desambiguación. |
| Negativo | `para cada x en Caja { xs: [1] }.xs { ... }` | Error sintáctico de desambiguación. |

### C2 — Lvalues compuestos (subconjunto v1)

| Tipo | Caso | Esperado |
|---|---|---|
| Positivo | `var p: Punto = Punto { x: 1, y: 2 }; p.x = 9` | Mutación de campo directo en variable mutable. |
| Positivo | `var xs: lista<entero> = [1,2]; xs[0] = 9` | Mutación índice directo en variable mutable. |
| Positivo | Efectos: RHS evalúa después de proyección/index; una sola mutación observable | Orden y atomicidad conforme. |
| Negativo | `p.x.y = 1` (encadenado) | Rechazo forma lvalue no soportada v1. |
| Negativo | `p.xs[0] = 1` | Rechazo forma lvalue no soportada v1. |
| Negativo | `xs[0].x = 1` | Rechazo forma lvalue no soportada v1. |
| Negativo | `xs[0][1] = 1` | Rechazo forma lvalue no soportada v1. |
| Negativo | `dato p: Punto = ...; p.x = 1` | Rechazo por inmutabilidad. |
| Negativo | `xs[-1] = 1` / fuera de rango | Error sin mutación parcial. |
| Diferencial | Todas las formas admitidas/rechazadas anteriores | Misma aceptación/rechazo/diagnóstico normalizado TW↔VM. |
| Semántica | Campo con handle Arc asignado/reemplazado | Preserva semántica de handle existente; sin clonar recurso subyacente. |

### C3 — Visibilidad y módulos

| Tipo | Caso | Esperado |
|---|---|---|
| Positivo | Tipo `publico estructura` importado por `usar` | Tipo visible y usable. |
| Positivo | Campo `publico` accedido desde módulo importador | Acceso permitido. |
| Positivo | Enum público: variantes accesibles como `Tipo.Variante` | Permitido. |
| Negativo | Acceso a campo privado desde otro módulo | BAS-7128. |
| Negativo | Uso de tipo no exportado/público desde otro módulo | BAS-7101/BAS-7128 según resolución. |
| Negativo | Impl duplicada `(Rasgo, Tipo)` en unidad de compilación | BAS-7122. |
| Negativo | Impl fuera de módulo del tipo y del rasgo | BAS-7122 (coherencia). |
| Integración | `exportar`/`publico`/`usar` existentes | Sin sistema paralelo; reglas integradas. |

## 2. Structs (S8.1)

### Positivos
- Declarar `estructura Punto { x: entero, y: entero }`.
- Construir con todos los campos en cualquier orden fuente.
- Acceder `p.x`.
- Asignar `p.x = ...` en lvalue mutable.
- Pasar/retornar struct (copia por valor).
- Struct vacío `estructura Marca {}`.
- Recursividad indirecta `estructura Nodo { hijos: lista<Nodo> }`.

### Negativos
- BAS-7101 tipo desconocido.
- BAS-7102 duplicado/sombreado.
- BAS-7103 campo inexistente.
- BAS-7104 campo duplicado.
- BAS-7105 campo faltante.
- BAS-7106 campo sobrante.
- BAS-7107 tipo incompatible.
- BAS-7128 campo no visible desde otro módulo.
- BAS-7130 recursión por valor directa/indirecta.
- Asignación a campo sobre binding inmutable (error de mutabilidad existente o diagnóstico normativo definido en implementación).

### Límite/efectos
- Inicializadores con efectos secundarios evaluados en orden de declaración, no en orden escrito.
- Copia: mutar copia no afecta original; mutar lvalue mutable sí afecta.

### IR
- HIR/MIR rechazan construcción con campo faltante/sobrante/tipo inválido.
- Proyección campo fuera de layout rechazada por verificador.

## 3. Enums/ADTs (S8.2)

### Positivos
- `enumeracion Opcion { Ninguno, Alguno(entero) }`.
- Construcción unitaria y payload posicional.
- Match por variante.
- Payload con nominales anidados.
- Recursión indirecta por `lista`.

### Negativos
- BAS-7109 variante inexistente.
- BAS-7110 aridad/tipo payload incompatible.
- BAS-7111 forma inválida o payload struct-like diferido.
- BAS-7130 recursión por valor.

### Diferenciales TW↔VM
- Construcción/match variante unitaria y con payload.
- Errores BAS-7109/7110 equivalentes.

## 4. Pattern matching (S8.3)

### Positivos
- `_`, binding, literales entero/logico/texto/nulo.
- Variante con subpatrones.
- Struct pattern con todos los campos.
- Anidamiento.
- `coincidir` como expresión con tipo común.

### Negativos
- BAS-7112 patrón incompatible.
- BAS-7113 no exhaustivo.
- BAS-7114 inalcanzable (error).
- BAS-7115 binding duplicado.
- BAS-7116 uso inválido.

### Exhaustividad obligatoria
- Enum sin variante faltante.
- Bool sin caso faltante.
- Tipo abierto sin `_`/binding irrefutable -> BAS-7113.
- Wildcard alcanzable cubre resto; wildcard inalcanzable -> BAS-7114.

### Efectos
- Scrutinee evaluado una sola vez.
- Solo brazo seleccionado evalúa efectos.

## 5. Métodos/impl (S8.4a)

### Positivos
- Método instancia `yo`.
- Método instancia `yo var` muta copia local.
- Método asociado `Tipo.crear(...)`.
- Llamada `obj.m()`.

### Negativos
- BAS-7117 método no encontrado.
- BAS-7118 firma incompatible o colisión campo/método/inherente-trait.
- BAS-7128 método no visible.

### Resolución
- Inherente antes que trait visible; ambigüedad/colisión prohibida v1.

## 6. Traits (S8.4b)

### Positivos
- Declarar `rasgo`, implementar completo para tipo concreto.
- Llamada a método de trait en tipo concreto.

### Negativos
- BAS-7119 trait no encontrado.
- BAS-7120 trait no implementado.
- BAS-7121 impl incompleta.
- BAS-7122 impl duplicada o coherencia.

### Diferenciales
- Resolución estática tipo concreto TW↔VM.

## 7. Igualdad y capacidades diferidas

- `==`/`!=` nominal -> BAS-7126.
- Nominal genérico -> BAS-7125.
- Backend no soportado -> BAS-7124 con backend/capa.

## 8. Integración G4 / no regresión

### Casos
- Función genérica recibe nominal concreto como tipo concreto: permitido si ya tipa.
- Función genérica con `Param T` intenta acceso campo/variante: rechazado por opacidad/reglas existentes, sin extender G4.
- No aceptar genéricos nominales.

### Regresión
- `genericos_frontend_g2` 23/23
- `genericos_semantica_g2_2` 20/20
- `genericos_semantica_g3` 55/55
- `genericos_monomorfizacion_g4` 33/33
- `diferencial_vm` 145/145
- workspace verde; 2 `registry_integration` ignoradas preexistentes clasificadas.

## 9. IR y verificadores

- HIR/MIR aceptan construcciones válidas.
- HIR/MIR rechazan: campo inválido, variante inválida, match no exhaustivo mal bajado, target inválido, proyección fuera de rango.
- Ningún caso IR inválido causa UB/pánico no diagnosticado.