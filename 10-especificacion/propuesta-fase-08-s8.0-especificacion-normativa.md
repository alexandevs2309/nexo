# Propuesta S8.0 R3 — Especificación normativa integral Fase 08

> **Estado:** APROBADA POR REVISIÓN HUMANA (2026-11-22) — S8.0 spec/contrato aprobado; **implementación de Fase 08 / S8.1 NO autorizada**  
> **Clasificación S8.0:** DESIGNED / DOCUMENTED / VERIFIED / APPROVED (human review) / READY FOR S8.1 AUTHORIZATION REQUEST  
> **No usar:** CLOSED, IMPLEMENTED  
> **Fecha de consolidación R3:** 2026-11-22  
> **Fecha de aprobación humana:** 2026-11-22  
> **Alcance:** Consolidación normativa S8.0 R3. Corrige bloqueos C1–C3 de la revisión independiente R2. **No autoriza implementación.** No autoriza S8.1.  
> **Dependencias:** Fase 06 APROBADA/CLOSED, Fase 07 APROBADA/CLOSED (G2.1–G6, commit `bb1bfedefeec6a1bc3eef2b93336abe36a25bfc4`).  
> **Gobernanza:** `AGENTS.md`, `DEVELOPMENT_STATE.md`, `ROADMAP_MASTER.md`. El contenido normativo R3 permanece sin cambios; solo se actualiza estado documental.

## 1. Propósito y regla de interpretación

Esta revisión R2 consolida los seis documentos S8.0 en un contrato normativo único. Resuelve las decisiones abiertas D8-01..D8-14, elimina contradicciones y fija la sintaxis/semántica canónica de Fase 08 v1.

**Regla de precedencia S8.0:** esta especificación es la fuente principal. Las decisiones arquitectónicas, matrices de capacidades/aceptación, propuesta de diagnósticos y plan S8.1–S8.5 deben derivar de este documento. Si hay conflicto entre documentos S8.0, prevalece esta especificación.

**Regla de aprobación:** este documento es una propuesta. Ninguna decisión aquí contenida se considera aprobada hasta revisión humana explícita.

## 2. Base arquitectónica confirmada (inspección 2026-11-21)

| Área | Evidencia | Hallazgo confirmado |
|---|---|---|
| Tokens/lexer | `prototipo/src/token.rs`, `lexer.rs` | No hay keywords/tokens para struct/enum/trait/impl/match nominal. Existen `Punto`, `DosPuntos`, `Flecha`, `Llave`, `Corchete`, `Usar/Como/Publico`, `Funcion`, `Retornar`. |
| AST/parser | `prototipo/src/parser.rs` | `Sentencia` carece de struct/enum/trait/impl/match. `Expresion` tiene `Acceso`, `LlamadaIndirecta`, `Indice`, `Lambda`. `TipoAnotacion` = primitivos, `Arreglo`, `Lista`, `Nulo`, `Parametro(String)`. |
| Semántica | `prototipo/src/semantico.rs` | `TipoSem` = primitivos, `Arreglo`, `Lista`, `Coleccion`, `Funcion{params,retorno,genericos}`, `Nulo`, `Param(TipoParamId)`, `Desconocido`. G2–G4 presentes. |
| Símbolos | `prototipo/src/simbodo.rs` | `SimbodoId(u32)`, `AmbitoId(u32)`, `CategoriaSimbodo`={Variable,Funcion,Parametro,Modulo,Builtin}; `TablaSimbodo` indexa por `(AmbitoId,String)`. |
| HIR | `prototipo/src/hir.rs` | Tiene `Acceso`, `LlamadaIndirecta`, `ModuloObjeto`, control, try, ciclos; sin construcción nominal/match/variante. |
| MIR | `prototipo/src/mir.rs` | Tiene `Campo`, `ObjetoModulo`, `SwitchInt`, `Invoke`, `Try`, proyecciones; sin discriminante nominal/match decision node. |
| Runtime | `prototipo/src/valor.rs` | `Valor::Objeto(BTreeMap<String,Valor>)` es dinámico (JSON/módulos). `Valor` es `Clone`; handles compartidos usan `Arc<Mutex<...>>`. |
| TW/VM | `evaluador.rs`, `vm.rs` | Objeto dinámico y SwitchInt soportados; sin nominales ADT/match. Asignación índice actualmente limitada a base variable. |
| G4 | `monomorfizacion.rs` | `GenericFnId`, `InstanceKey`, `TipoCanon`, sustitución, worklist, M1–M3, gate integridad. CLOSED. |
| Diagnósticos | `catalogo-de-errores-y-diagnosticos.md` | BAS-70xx hasta 7021 usado/reservado. Verificación R2: no hay entradas BAS-71xx/BAS-72xx/BAS-73xx/BAS-80xx en catálogo. |
| Brechas | `registro-de-brechas.md` | GAP-035 abierta (estructuras/nominales ausentes). |

Matices: existe infraestructura reusable (Acceso/Campo/SwitchInt/SimbodoId/verificadores), pero no se reutiliza `Valor::Objeto` como nominal. `SimbodoId` usa `saturating_add`; Fase 08 debe definir error de capacidad si se agota.

## 3. Principios normativos

1. **Nominalidad estricta:** compatibilidad por identidad nominal, no por estructura de campos.
2. **Separación dinámica/nominal:** `Objeto` dinámico != struct/enum nominal.
3. **Determinismo:** evaluación, lowering, exhaustividad y diagnósticos deterministas.
4. **Paridad declarada:** TW↔VM obligatoria en subconjunto aprobado. Nativo/WASM por capacidad con evidencia.
5. **Integridad IR:** toda operación nominal/match representable y verificable en HIR/MIR.
6. **G4 intacto:** no modificar M1–M3, `GenericFnId`, `InstanceKey`, `TipoCanon`, sustitución, worklist, gate integridad.
7. **Sin semántica implícita:** sin coerciones nominales, sin dispatch dinámico, sin referencias/borrowing implícitos, sin inferencia global oculta.
8. **Diagnóstico antes que pánico:** errores de usuario/problemas de capacidad declarada producen diagnósticos estables.

## 4. Decisiones normativas resueltas (D8-01..D8-14)

| ID | Decisión R2 | Estado |
|---|---|---|
| D8-01 | Identidad nominal estable por `TipoNominalId` + módulo + nombre canónico + clase; no textual/estructural. | Resuelta en propuesta |
| D8-02 | `Valor::Objeto` se mantiene dinámico; nominales usan `StructNominal`/`EnumNominal` conceptuales. | Resuelta en propuesta |
| D8-03 | Sintaxis canónica: `estructura`, `enumeracion`, `coincidir`, `rasgo`, `implementar`; receptor `yo`/`yo var`; sin aliases en v1. | Resuelta en propuesta |
| D8-04 | Mutabilidad por binding/lvalue; valor-semantics con copia; sin referencias implícitas; `yo var` muta copia local. | Resuelta en propuesta |
| D8-05 | `==`/`!=` nominal no soportado en v1; diagnóstico BAS-7126. | Resuelta en propuesta |
| D8-06 | Structs: campos obligatorios, construcción completa, evaluación de inicializadores en orden de declaración, lvalues extendidos por proyección. | Resuelta en propuesta |
| D8-07 | Enums: variantes unitarias y payload posicional en v1; struct-like payload diferido. | Resuelta en propuesta |
| D8-08 | `coincidir` es expresión; exhaustividad obligatoria; algoritmo de cobertura definido; unreachable = error BAS-7114. | Resuelta en propuesta |
| D8-09 | Impl inherente con métodos de instancia y asociados; resolución estática; colisiones prohibidas. | Resuelta en propuesta |
| D8-10 | Traits estáticos mínimos; sin trait objects/dyn/default/supertraits/associated types/bounds/impl genéricas. | Resuelta en propuesta |
| D8-11 | G4 intacto; nominales genéricos diferidos; nominal concreto permitido donde ya es tipo concreto. | Resuelta en propuesta |
| D8-12 | Recursividad por valor prohibida; recursividad indirecta por `lista<T>`/`arreglo<T,n>` permitida. | Resuelta en propuesta |
| D8-13 | Visibilidad nominal definida: tipo público/privado; campos privados por defecto; variantes siguen visibilidad del enum; métodos/traits por reglas explícitas. | Resuelta en propuesta |
| D8-14 | Matriz por capacidad y por capa (semántica/IR/codegen/ejecución); TW↔VM obligatorio; Nativo/WASM declarados. | Resuelta en propuesta |

## 5. D8-01 — Identidad nominal, ámbitos y resolución

### 5.1 Identidad

Cada tipo nominal (`estructura`, `enumeracion`, `rasgo`) tiene identidad estable:

- `TipoNominalId`: identificador interno estable del programa analizado (entero monotónico por compilación/análisis; no reutilizado tras asignación). Agotamiento: error interno/diagnóstico de capacidad de compilación (no pánico silencioso).
- `nombre_canonico`: nombre declarado.
- `modulo_id`/ruta lógica de módulo: ámbito de definición.
- `clase`: Struct | Enum | Trait.
- `parametros_genericos`: vacío en v1.

**Compatibilidad:** dos tipos son compatibles solo si su `TipoNominalId` es igual. No hay compatibilidad estructural.

### 5.2 Espacios de nombres

Separar al menos:

- Espacio de **tipos nominales** (struct/enum/trait).
- Espacio de **valores/funciones/variables**.
- Espacio de **campos** por tipo struct.
- Espacio de **variantes** por enum.
- Espacio de **métodos** por tipo (inherentes y de trait, con reglas de colisión).

Un nombre puede existir en distinto espacio sin conflicto (p. ej. variable `Punto` y tipo `Punto` solo si el lenguaje ya lo permite; v1: **prohibido** para evitar ambigüedad en sintaxis de construcción/patrones, diagnóstico de colisión).

### 5.3 Resolución y ámbitos

- Declaraciones nominales se recolectan en pase de declaración de módulo antes de resolver cuerpos, para permitir referencias adelantadas y recursividad indirecta.
- Resolución de tipo: ámbito de módulo actual, luego imports visibles (`usar`/`importar` según reglas vigentes). No búsqueda implícita en módulos no importados.
- Duplicado en mismo ámbito/módulo: BAS-7102.
- Sombreado de tipo nominal en ámbito interno: **prohibido en v1** (BAS-7102 o diagnóstico específico de sombreado; se usa BAS-7102 con detalle "sombreado no permitido en v1").
- Referencias a tipos declarados después en el mismo módulo: permitidas.

### 5.4 Visibilidad y módulos (C3, normativa)

**Anclaje a mecanismos existentes:** BASILISCO ya tiene `publico` en declaraciones/funciones, `exportar` como sentencia envolvente, `usar`/`importar` y reglas de exportación en semántico/cargador. Fase 08 no crea un sistema de módulos paralelo; extiende estos mecanismos.

**Reglas v1:**

| Elemento | Por defecto | `publico` | Exportación/importación | Regla |
|---|---|---|---|---|
| `estructura`/`enumeracion`/`rasgo` | privado al módulo | visible fuera del módulo | Sigue reglas existentes de exportación de símbolos públicos del módulo; `exportar` sobre declaración nominal es equivalente a marcarla pública si el lenguaje ya lo permite | Tipo no visible: BAS-7101/BAS-7128 según contexto |
| Campo de struct | privado al módulo | visible a módulos que pueden ver el tipo | No se exporta individualmente; visibilidad condicionada al tipo | Acceso no visible: BAS-7128 |
| Variante de enum | sigue visibilidad del enum | N/A en v1 | No visibilidad por variante | Variante de enum visible accesible como `Tipo.Variante` si el tipo es visible |
| Método inherente | privado al módulo | visible fuera del módulo | Condicionado a visibilidad del tipo | BAS-7128 si no visible |
| Método de rasgo | público por contrato del rasgo público | N/A | Sigue visibilidad del rasgo | Rasgo privado: no usable fuera |
| `implementar Rasgo para Tipo` | sin nombre global | N/A | Impl no es símbolo importable por nombre; su efecto está disponible cuando tipo y rasgo son visibles y la impl es coherente | — |

**Coherencia local de impl (v1):** una `implementar Rasgo para Tipo` debe declararse en el módulo que declara `Tipo` o en el módulo que declara `Rasgo`. Fuera de ambos: BAS-7122 (detalle coherencia). Esto evita reglas de huérfanos complejas en v1.

**Duplicados de impl:** máximo una impl por par `(TipoNominalId, RasgoNominalId)` en la unidad de compilación del programa analizado. La detección se realiza en la fase de recolección semántica/global del programa (antes de ejecución/codegen), no como capacidad ya existente del runtime. Duplicado: BAS-7122.

**Requisito de implementación (no capacidad existente):** el soporte de visibilidad nominal en módulos es requisito de S8.1/S8.4, no una capacidad ya presente en el runtime actual. La implementación debe integrarse con `cargador`/semántica de módulos existente.

### 5.5 HIR/MIR/Runtime

- HIR y MIR transportan `TipoNominalId`/identidad canónica resuelta, no nombre textual sin resolver.
- TW y VM comparan identidad nominal por ID canónico, no por nombre corto.
- Serialización/debug puede incluir nombre canónico legible, pero la identidad semántica es el ID nominal.

## 6. D8-02 — Representación runtime

### 6.1 Separación

- `Valor::Objeto(BTreeMap<String,Valor>)`: permanece dinámico. Sin tag nominal.
- `Valor::StructNominal`: valor nominal struct.
- `Valor::EnumNominal`: valor nominal enum/ADT.

### 6.2 Contrato conceptual

```text
StructNominal {
  tipo: TipoNominalId,
  campos: LayoutOrdenadoValores   // orden canónico por declaración
}

EnumNominal {
  tipo: TipoNominalId,
  variante: VarianteId,
  payload: Vec<Valor>             // orden posicional
}
```

**Propiedad:** la representación física puede ser `Vec<Valor>`, arreglo fijo, o layout especializado por backend, siempre que preserve: identidad nominal, identidad de variante, orden de campos/payload, valor de cada elemento, y semántica de copia/mutabilidad.

### 6.3 Layout y acceso

- Orden de campos = orden de declaración en struct.
- Acceso semántico por nombre se resuelve en compilación a índice/proyección estable.
- Runtime puede usar índice; debug puede mostrar nombres.
- Payload de variante posicional: orden declarado.

### 6.4 Copia, transferencia y handles

- Asignar/pasar/retornar nominal copia el `Valor` según semántica existente de `Valor::Clone`.
- Componentes con handles compartidos (`Archivo`, `Celda`, `FuncionClausura`, `Modulo`, etc.) conservan su semántica actual de compartición por `Arc`; Fase 08 no cambia eso.
- Campos nominales que contienen dichos handles copian el handle, no clonan el recurso subyacente, igual que hoy.

### 6.5 Debug/serialización

- Debug textual estable: `Tipo { campo: valor, ... }`, `Tipo.Variante(...)`, `Tipo.Variante`.
- No es contrato de parseo.
- JSON/serialización automática de nominales: **no definida en v1**. Cualquier intento de serialización automática donde no haya contrato explícito debe producir error/diagnóstico de capacidad (BAS-7124 o diagnóstico específico de serialización). No serializar nominal como `Objeto` dinámico implícitamente.

## 7. D8-03 — Sintaxis canónica v1

### 7.1 Palabras clave canónicas

| Concepto | Canónico v1 | No canónico en v1 |
|---|---|---|
| Struct | `estructura` | `struct` |
| Enum | `enumeracion` | `enum` |
| Match | `coincidir` | `match`, `segun` |
| Trait | `rasgo` | `trait` |
| Impl | `implementar` | `impl` |
| Receptor | `yo` | `self`, `este` |

No se aceptan aliases en v1. Los tokens/keywords canónicos deben reservarse.

### 7.2 Gramática normativa (EBNF orientativa)

```ebnf
decl_tipo      ::= decl_estructura | decl_enumeracion | decl_rasgo | decl_impl ;
decl_estructura ::= vis? "estructura" Ident "{" campo_struct* "}" ;
campo_struct   ::= vis? Ident ":" tipo ";"? ;
decl_enumeracion ::= vis? "enumeracion" Ident "{" variante_enum* "}" ;
variante_enum  ::= Ident payload_pos? ";"? ;
payload_pos    ::= "(" tipo ("," tipo)* ","? ")" ;
decl_rasgo     ::= vis? "rasgo" Ident "{" firma_metodo* "}" ;
firma_metodo   ::= "funcion" Ident "(" params_metodo? ")" ("->" tipo)? ";"? ;
decl_impl      ::= "implementar" Ident "{" metodo_impl* "}"
                 | "implementar" Ident "para" Ident "{" metodo_impl* "}" ;
metodo_impl    ::= vis? "funcion" Ident "(" params_metodo? ")" ("->" tipo)? bloque ;
params_metodo  ::= receptor ("," param)* | param ("," param)* ;
receptor       ::= "yo" | "yo" "var" ;
constr_struct  ::= Ident "{" (init_campo ("," init_campo)* ","?)? "}" ;
init_campo     ::= Ident ":" expr ;
constr_variante ::= Ident "." Ident | Ident "." Ident "(" args? ")" ;
expr_coincidir ::= "coincidir" expr "{" brazo+ "}" ;
brazo          ::= patron "=>" (expr | bloque) ","? ;
patron         ::= "_" | Ident | literal | patron_variante | patron_struct ;
patron_variante ::= Ident "." Ident ("(" patrones? ")")? ;
patron_struct  ::= Ident "{" (campo_patron ("," campo_patron)* ","?)? "}" ;
campo_patron   ::= Ident ":" patron ;
```

La gramática final se ajustará al parser real en S8.1, pero esta forma es el contrato normativo de S8.0.

### 7.3 Desambiguación sintáctica (C1, normativa)

**Problema:** `{ ... }` inicia bloques en producciones de control (`si`, `mientras`, `para cada`, `coincidir`). La construcción struct `Ident { ... }` usa la misma delimitación.

**Regla normativa única:** en posiciones de *expresión de cabeza* donde la gramática espera `expresión "{" bloque`, la forma directa `Ident { ... }` (construcción struct) está **prohibida** como expresión externa de esa cabeza. La expresión debe envolverse en paréntesis de agrupación `( ... )` o reescribirse sin literal struct directo.

Posiciones afectadas en v1:

| Producción | Cabeza | Prohibido directo | Forma válida |
|---|---|---|---|
| `si <expr> { ... }` | condición | `si Punto { x: 1 } { ... }` | `si (Punto { x: 1 }).activo { ... }` o `si (es_valido(Punto { x: 1 })) { ... }` |
| `mientras <expr> { ... }` | condición | `mientras Estado { ok: verdadero } { ... }` | `mientras (Estado { ok: verdadero }).ok { ... }` |
| `para cada id en <expr> { ... }` | iterable | `para cada x en Caja { xs: [1] }.xs { ... }` | `para cada x en (Caja { xs: [1] }).xs { ... }` |
| `coincidir <expr> { ... }` | scrutinee | `coincidir Opcion { ... }` (si fuera struct literal) / `coincidir Punto { x: 1 } { ... }` | `coincidir (Punto { x: 1 }) { ... }` |

**Alcance:** la prohibición aplica solo a la construcción literal struct `Ident "{" ... "}"` en la posición de cabeza externa. No prohíbe `coincidir` como expresión (que tiene sus propios brazos), ni prohíbe struct literals dentro de paréntesis, argumentos, índices, campos, o brazos de `coincidir`.

**Compatibilidad parser:** regla implementable con lookahead/restricción de producción en parser descendente recursivo; no requiere cambiar semántica de bloques.

**Casos de aceptación documental:**
- Válido: `var p: Punto = Punto { x: 1, y: 2 }`
- Válido: `si (Punto { x: 1, y: 2 }).x > 0 { mostrar("ok") }`
- Válido: `coincidir (Opcion.Alguno(1)) { Opcion.Alguno(v) => v, Opcion.Ninguno => 0 }`
- Inválido (diagnóstico sintáctico): `si Punto { x: 1, y: 2 }.x > 0 { ... }`
- Inválido (diagnóstico sintáctico): `coincidir Punto { x: 1, y: 2 } { ... }`

### 7.4 Ejemplos normativos

```basilisco
estructura Punto {
    publico x: entero
    publico y: entero
}

enumeracion Opcion {
    Ninguno
    Alguno(entero)
}

funcion origen() -> Punto {
    retornar Punto { x: 0, y: 0 }
}

funcion mover(p: Punto, dx: entero) -> Punto {
    var q: Punto = p
    q.x = q.x + dx
    retornar q
}

funcion describir(o: Opcion) -> texto {
    retornar coincidir o {
        Opcion.Ninguno => "nada"
        Opcion.Alguno(v) => "valor"
    }
}
```

```basilisco
rasgo Mostrable {
    funcion mostrar_texto(yo) -> texto
}

implementar Punto {
    funcion suma(yo) -> entero {
        retornar yo.x + yo.y
    }
}

implementar Mostrable para Punto {
    funcion mostrar_texto(yo) -> texto {
        retornar "Punto"
    }
}
```

## 8. D8-04 — Mutabilidad, copia, lvalues y receptores

### 8.1 Modelo de valor

- Structs y enums nominales son **valores**. Se copian en asignación, paso de argumento, retorno e inserción en contenedores, conforme a `Valor::Clone`.
- No hay referencias implícitas, borrowing ni aliasing mutable implícito de campos nominales.
- Handles compartidos preexistentes mantienen su semántica actual.

### 8.2 Bindings

- `var` / `sea`: mutable.
- `dato` / `const`: inmutable.
- La mutabilidad es propiedad del binding/lvalue, no de un campo declarado en v1.

### 8.3 Lvalues v1 (C2, normativa)

**Subconjunto admitido en v1 (exhaustivo):**

| Forma | Admitida v1 | Ejemplo | Notas |
|---|---:|---|---|
| `id = expr` | Sí | `p = otro` | Asignación variable existente, ya soportada. |
| `id.campo = expr` | Sí | `p.x = 1` | Campo directo sobre variable mutable. |
| `id[indice] = expr` | Sí | `xs[0] = 1` | Índice directo sobre variable mutable, ya soportado. |
| `id.campo.campo = expr` | No | `a.b.c = 1` | Proyección encadenada diferida. |
| `id.campo[indice] = expr` | No | `p.xs[0] = 1` | Proyección encadenada diferida. |
| `id[indice].campo = expr` | No | `xs[0].x = 1` | Proyección encadenada diferida. |
| `id[indice][indice] = expr` | No | `m[0][1] = 1` | Índice encadenado diferido. |
| Cualquier base no identificador | No | `f().x = 1` | Temporal no asignable. |

Las formas no admitidas producen error sintáctico o diagnóstico semántico de "lvalue no soportado en Fase 08 v1" (BAS-7131 reservado para implementación; hasta asignación, usar BAS-7108/BAS-7124 según capa, con mensaje explícito de forma no soportada). **No prometer soporte encadenado en v1.**

**Evaluación y atomicidad observable (formas admitidas):**

1. **Resolver base:** localizar identificador. Debe existir y ser mutable. Si no, error; sin mutación.
2. **Evaluar componente de proyección (si existe), una sola vez, izquierda a derecha:** para `id[indice]`, evaluar `indice` a entero. Para `id.campo`, resolver campo en tipo estático/conocido. Si falla, error; sin mutación.
3. **Evaluar RHS:** evaluar `expr` una vez. Si falla, error; sin mutación.
4. **Validar:** tipo compatible, bounds, visibilidad, mutabilidad. Si falla, error; sin mutación.
5. **Aplicar mutación:** escribir valor en la ubicación objetivo. La mutación es la única escritura observable.

**Atomicidad observable:** para las formas admitidas, la asignación es todo-o-nada respecto al valor objetivo. No se permite estado parcialmente mutado observable por el usuario ante error en pasos 1-4. El paso 5 no debe fallar por validación ya comprobada salvo error interno.

**Copia y handles Arc:** la mutación reemplaza el valor almacenado en la ubicación objetivo. Si el valor contiene handles `Arc<Mutex<...>>` (Archivo, Celda, FuncionClausura, Módulo, etc.), se preserva la semántica actual de handles compartidos; copiar/asignar handle no clona el recurso subyacente.

**Paridad:** TW↔VM deben producir el mismo resultado o el mismo diagnóstico normalizado para las mismas formas admitidas/rechazadas.

### 8.4 Asignación de campo

`p.campo = expr` es válida solo si:

- `p` es lvalue mutable.
- `campo` existe y es visible.
- tipo de `expr` compatible con tipo del campo.

### 8.5 Efectos y orden

- Inicializadores de struct: se evalúan **en orden de declaración de campos**, de izquierda a derecha. El literal puede escribir campos en cualquier orden fuente, pero debe incluir todos los campos exactamente una vez.
- Argumentos de llamada: mantienen regla existente del lenguaje (evaluación izquierda a derecha; si hoy existe otra regla, se respeta la regla actual y se documenta en implementación, pero S8.0 recomienda consistencia izquierda a derecha).
- Construcción de variante: argumentos de payload evaluados izquierda a derecha.

### 8.6 Receptores

- `yo`: binding inmutable del receptor, copia del valor receptor.
- `yo var`: binding mutable local del receptor, copia mutable local; mutaciones no afectan al llamador.
- v1 no incluye `yo ref`/referencias. Mutar el valor original se hace mediante asignación a lvalue externo o devolviendo un nuevo valor.

## 9. D8-05 — Igualdad

### 9.1 Regla v1

Para structs/enums nominales, `==` y `!=` **no están soportados en v1**.

- Emitir BAS-7126 `IgualdadNominalNoSoportada`.
- No comparar por identidad estructural.
- No comparar por nombre textual.
- No comparar por dirección/handle implícito.

### 9.2 Tipos nominales distintos

La comparación entre nominales distintos también es inválida y produce BAS-7126 o BAS-7127 según capa; norma: BAS-7126 para operador no soportado en nominales v1, BAS-7127 reservado para incompatibilidad de tipos general en comparaciones futuras. En v1, usar BAS-7126 cuando cualquiera de los operandos sea nominal.

### 9.3 Evolución

Fase futura puede introducir igualdad derivada/trait. No se asume en v1.

## 10. D8-06 — Structs

### 10.1 Declaración

- `estructura Nombre { ... }`.
- Campos únicos por struct.
- Tipo explícito por campo.
- Struct vacío permitido.
- Visibilidad por campo: privado por defecto, `publico` opcional.

### 10.2 Construcción

- Requiere todos los campos.
- Sin campos extra, sin duplicados.
- Orden de evaluación: orden de declaración (determinista).
- `Nombre {}` válido solo para struct vacío.

### 10.3 Acceso

- `obj.campo` solo si tipo estático es struct nominal con campo visible en contexto.
- Campo inexistente/invisible: BAS-7103 o BAS-7128 (visibilidad). Se define BAS-7128 `MiembroNoVisible`.

### 10.4 Asignación

- Solo sobre lvalue mutable.
- Tipo de campo compatible.

### 10.5 Recursividad

- Prohibida recursividad por valor directa o indirecta.
- Permitida recursividad indirecta por `lista<T>` o `arreglo<T,n>`.
- Ciclos se detectan en grafo de dependencias de layout por valor.

## 11. D8-07 — Enums y ADTs

### 11.1 Declaración

- `enumeracion Nombre { ... }`.
- Variantes únicas.
- v1 soporta variantes unitarias y payload posicional.
- Payload struct-like diferido (BAS-7125 o BAS-7111 por forma no soportada en v1; usar BAS-7111 con detalle "payload struct-like diferido en v1").

### 11.2 Construcción

- Unitaria: `Tipo.Variante`.
- Payload: `Tipo.Variante(expr, ...)`.
- Aridad y tipos deben coincidir exactamente.

### 11.3 Discriminante

- Identidad lógica de variante; sin discriminantes numéricos de usuario en v1.
- Representación interna entera compacta permitida.

### 11.4 Recursividad

Mismas reglas que structs: por valor prohibida, indirecta por `lista/arreglo` permitida.

## 12. D8-08 — Pattern matching y exhaustividad

### 12.1 Forma

`coincidir` es una **expresión**. Su valor es el valor del brazo seleccionado. Cuando aparece como sentencia de expresión, el valor se descarta.

### 12.2 Evaluación

1. Evaluar scrutinee una vez a valor.
2. Probar brazos en orden fuente.
3. Seleccionar primer brazo cuyo patrón coincide y (futuro) guard pase; v1 no tiene guards.
4. Evaluar solo el brazo seleccionado.
5. Si ningún brazo coincide, es error de no exhaustividad detectado en compilación; no hay fallback runtime en match exhaustivo válido.

### 12.3 Patrones v1

| Patrón | v1 | Irrefutable | Notas |
|---|---|---|---|
| `_` | Sí | Sí | No enlaza. |
| Binding `id` | Sí | Sí, salvo tipos cerrados donde se use como cobertura parcial | Enlaza copia/valor según semántica Valor. |
| Literal entero/logico/texto/nulo | Sí | No | Igualdad del literal existente. |
| Variante enum | Sí | No, salvo cobertura total | `Tipo.Variante(pats...)`. |
| Struct | Sí | No, salvo cobertura total | `Tipo { campo: pat, ... }`, todos los campos requeridos o `_` por campo explícito; no hay `..` en v1. |
| Guard | No | N/A | Diferido. |
| Or-pattern | No | N/A | Diferido. |
| Rango/lista | No | N/A | Diferido. |

### 12.4 Bindings

- Locales al brazo.
- No duplicados en el mismo patrón (BAS-7115).
- No sombrean de forma ambigua fuera de regla; sombreado local normal de bloque permitido, pero dentro del mismo patrón prohibido.

### 12.5 Exhaustividad

Obligatoria en toda expresión `coincidir`.

Algoritmo normativo (suficiente para implementación y pruebas):

1. **Normalizar patrón** a matriz de columnas por constructor/campo.
2. **Tipos cerrados:** enumeracion = conjunto de variantes. booleano = {verdadero,falso}. 
3. **Tipos abiertos:** entero, texto, lista, arreglo, objeto, función, nominales sin cobertura cerrada total = requieren fila comodín/irrefutable.
4. **Cobertura variante:** cubierta si todos sus subpatrones están cubiertos para el espacio de payload.
5. **Cobertura struct:** cubierta si todos los campos están cubiertos.
6. **Cobertura literal:** unión de literales específicos; completa solo con comodín/binding irrefutable.
7. **Resultado:** completo, incompleto (BAS-7113 con casos faltantes), redundante/inalcanzable (BAS-7114).

No se acepta "hay wildcard en algún brazo" como prueba suficiente salvo que el wildcard sea efectivamente alcanzable y cubra el espacio restante.

### 12.6 Tipado de brazos

- Todas las ramas deben producir tipos compatibles con un tipo común de resultado.
- Si se usa como sentencia de expresión, el tipo resultante puede descartarse, pero sigue validándose la compatibilidad interna.

## 13. D8-09 — Métodos inherentes e impl

### 13.1 Impl inherente

```basilisco
implementar Punto {
    publico funcion suma(yo) -> entero { retornar yo.x + yo.y }
    funcion crear(x: entero, y: entero) -> Punto { retornar Punto { x: x, y: y } }
}
```

- Método de instancia: primer parámetro receptor `yo` o `yo var`.
- Método asociado: sin receptor. Se llama `Tipo.metodo(...)`.
- Resolución estática.

### 13.2 Reglas

- No duplicar nombre de método en el mismo impl.
- No colisionar campo y método con mismo nombre en el tipo.
- Visibilidad de método: privada por defecto, `publico` opcional.
- Llamada `obj.m(...)`: resuelve primero métodos inherentes del tipo estático; si no existe, busca métodos de traits visibles/aplicables; ambigüedad = error.

### 13.3 Conflicto inherente/trait

Si un método inherente tiene el mismo nombre que un método de trait para el mismo tipo, v1: **error de colisión** (BAS-7118/BAS-7129 reservado; usar BAS-7118 con detalle) para evitar ambigüedad. Evolución futura puede definir desambiguación explícita.

## 14. D8-10 — Traits estáticos mínimos

### 14.1 Declaración

```basilisco
rasgo Mostrable {
    funcion mostrar_texto(yo) -> texto
}
```

- Solo firmas de métodos requeridos en v1.
- Sin default methods, sin associated types/constants, sin supertraits.

### 14.2 Implementación

```basilisco
implementar Mostrable para Punto {
    funcion mostrar_texto(yo) -> texto { retornar "Punto" }
}
```

- Debe implementar todos los métodos requeridos.
- Firmas compatibles: mismo nombre, misma aridad, tipos de parámetros y retorno compatibles según reglas nominales/actuales.
- Una impl por (Rasgo, Tipo) en unidad de compilación: duplicado BAS-7122.

### 14.3 Resolución

- Estática, basada en tipo concreto conocido.
- No trait objects, no dispatch dinámico.
- Uso de método de trait sobre tipo concreto: permitido si el trait está en alcance y el tipo implementa el trait.

### 14.4 Coherencia v1

Para evitar ambigüedad sin orphan rules complejas: en v1, una `implementar Rasgo para Tipo` debe estar en el módulo que declara el `Rasgo` o en el módulo que declara el `Tipo`. Impl fuera de ambos: BAS-7122/diagnóstico de coherencia (usar BAS-7122 con detalle "coherencia").

## 15. D8-11 — Compatibilidad con G4

### 15.1 Contratos intactos

No se modifican: `GenericFnId`, `InstanceKey`, `TipoCanon`, sustitución estructural, worklist, M1, M2, M3, gate integridad, diagnósticos BAS-70xx.

### 15.2 Nominales en funciones genéricas

- Un tipo nominal concreto (sin parámetros genéricos) es tipo concreto. Puede usarse como argumento/retorno/valor dentro de funciones genéricas cuando la instanciación G4 lo concreta normalmente.
- Los parámetros de tipo `Param` siguen opacos. No se puede acceder a campos, variantes, métodos o construir nominales que dependan de capacidades desconocidas del Param, salvo que una futura extensión G4-ext lo especifique.
- Genéricos nominales (`estructura Caja<T>`, `enumeracion Opcion<T>`) están **diferidos** en v1. Uso produce BAS-7125.

### 15.3 Extensión futura

Cualquier soporte de nominales genéricos, bounds, trait bounds o métodos genéricos requiere spec G4-ext aprobada. No se implementa por inferencia en S8.1–S8.5.

## 16. D8-12 — Recursividad y layout

- **Prohibido:** struct/enum que se contiene a sí mismo por valor directa o indirectamente sin indirection.
- **Permitido:** recursividad a través de `lista<T>` y `arreglo<T,n>`, porque el almacenamiento runtime actual es indirecto en `Valor` (Vec/contenerización de valores), y el layout por valor del nominal no se expande infinitamente.
- **Detección:** grafo de dependencia de campos/payloads por valor; ciclo = BAS-7130 `RecursividadNominalPorValor`.
- La comprobación se realiza en semántica tras recolección de declaraciones.

## 17. D8-13 — Visibilidad y módulos (resumen operativo)

Ver §5.4 para regla normativa completa (C3). Resumen:

| Elemento | Por defecto | `publico` | Regla |
|---|---|---|---|
| estructura/enumeracion/rasgo | privado al módulo | visible fuera del módulo | Integrado con `publico`/`exportar`/`usar` existentes; no sistema paralelo |
| campo struct | privado al módulo | visible fuera | Solo si tipo visible; BAS-7128 |
| variante enum | sigue visibilidad del enum | N/A en v1 | No visibilidad por variante |
| método impl | privado al módulo | visible fuera | Solo si tipo visible; BAS-7128 |
| método trait | público por contrato del rasgo público | N/A | Visible con rasgo |
| impl Rasgo para Tipo | sin nombre global | N/A | Coherencia local: módulo del tipo o del rasgo; duplicado/coherencia BAS-7122 |

Detección de impl duplicada: fase de recolección semántica del programa analizado, unidad de compilación completa del programa.

## 18. D8-14 — Backends y matriz por capa

### 18.1 Capas

Toda capacidad se clasifica en:

1. **Semántica/Tipos:** parse, resolución, tipado, diagnósticos.
2. **HIR/MIR:** representación y verificación.
3. **Codegen:** generación nativa/WASM.
4. **Ejecución:** runtime TW/VM/Nativo/WASM.

### 18.2 Regla

- TW y VM: obligatorios para capacidades aprobadas en v1.
- Nativo/WASM: soporte declarado por capacidad y por capa. No soporte = BAS-7124 en intento de compilación/ejecución no soportado.
- Una capacidad puede ser semánticamente válida pero no soportada en codegen nativo/WASM; debe rechazarse limpiamente.

## 19. HIR/MIR y verificadores

### 19.1 Nodos mínimos conceptuales

HIR:

- Declaraciones: `StructDecl`, `EnumDecl`, `TraitDecl`, `ImplInherenteDecl`, `ImplTraitDecl`.
- Expresiones: `ConstruirStruct`, `ConstruirVariante`, `AccesoCampoNominal`, `AsignarCampoNominal` (si se modela como expr/stmt), `LlamadaMetodo`, `Coincidir`.
- Patrones: `PatronWildcard`, `PatronBinding`, `PatronLiteral`, `PatronVariante`, `PatronStruct`.

MIR:

- `Rvalue::ConstruirStruct { tipo, campos }`
- `Rvalue::ConstruirVariante { tipo, variante, payload }`
- `Rvalue::LeerCampoNominal { base, tipo, indice_campo }`
- `Stmt::EscribirCampoNominal { base, tipo, indice_campo, valor }`
- `Rvalue::Discriminante { base, tipo_enum }`
- Match lowering: árbol de decisión con `SwitchInt` (discriminantes/literales enteros/bool), comparaciones, Goto, proyecciones.

Los nombres pueden variar en implementación; las invariantes no.

### 19.2 Invariantes de verificador

HIR verifier:

- Toda identidad nominal resuelta existe.
- Construcción struct incluye exactamente campos declarados, tipos compatibles.
- Construcción variante válida, aridad/tipos compatibles.
- Acceso campo visible/existente.
- Match tipado, exhaustivo, patrones compatibles.
- Llamada método resuelta sin ambigüedad.

MIR verifier:

- Tipo nominal/variante/campo índices válidos.
- Discriminante usado solo sobre enum nominal válido.
- SwitchInt targets válidos.
- No caída no manejada en match exhaustivo compilado.
- Proyecciones nominales dentro de rango.

Fallo de IR: error interno/verificación, no comportamiento indefinido.

## 20. Diagnósticos (propuesta normativamente clasificada)

Rango candidato **BAS-71xx**, verificado sin colisiones en catálogo actual (2026-11-21). Asignación final al aprobar.

| Código | Nombre | Clase | v1 | Emisión |
|---|---|---:|---|---|
| BAS-7101 | TipoNominalDesconocido | semántico | activo | tipo nominal no resuelto |
| BAS-7102 | TipoNominalDuplicadoOSombreado | semántico | activo | duplicado/sombreado no permitido |
| BAS-7103 | CampoStructDesconocido | semántico | activo | campo inexistente |
| BAS-7104 | CampoStructDuplicado | semántico | activo | campo duplicado en literal |
| BAS-7105 | CampoStructFaltante | semántico | activo | falta campo |
| BAS-7106 | CampoStructSobrante | semántico | activo | campo extra |
| BAS-7107 | TipoCampoIncompatible | semántico | activo | tipo campo incompatible |
| BAS-7108 | ConstruccionNominalInvalida | semántico | activo | forma general inválida |
| BAS-7109 | VarianteEnumDesconocida | semántico | activo | variante inexistente |
| BAS-7110 | PayloadVarianteIncompatible | semántico | activo | aridad/tipo payload inválido |
| BAS-7111 | ConstruccionVarianteInvalida | semántico | activo | forma variante inválida/diferida |
| BAS-7112 | PatronIncompatible | semántico | activo | patrón no compatible con scrutinee |
| BAS-7113 | MatchNoExhaustivo | semántico | activo | cobertura incompleta |
| BAS-7114 | PatronInalcanzable | semántico | activo | patrón redundante/inalcanzable |
| BAS-7115 | BindingPatronInvalido | semántico | activo | binding duplicado/inválido |
| BAS-7116 | UsoPatronInvalido | semántico | activo | patrón fuera de contexto |
| BAS-7117 | MetodoNoEncontrado | semántico | activo | método inexistente |
| BAS-7118 | FirmaMetodoIncompatibleOColision | semántico | activo | firma incompatible/colisión método |
| BAS-7119 | TraitNoEncontrado | semántico | activo | trait inexistente |
| BAS-7120 | TraitNoImplementado | semántico | activo | impl requerida ausente |
| BAS-7121 | ImplIncompleta | semántico | activo | método requerido faltante |
| BAS-7122 | ImplDuplicadaOCoherencia | semántico | activo | impl duplicada/violación coherencia |
| BAS-7123 | BoundNoSatisfecho | semántico | reservado | futuro bounds |
| BAS-7124 | CapacidadNominalNoSoportadaBackend | capacidad/backend | activo | backend no soporta capacidad |
| BAS-7125 | GenericoNominalNoSoportado | semántico/capacidad | activo | nominal genérico diferido en v1 |
| BAS-7126 | IgualdadNominalNoSoportada | semántico | activo | `==/!=` nominal no soportado v1 |
| BAS-7127 | ComparacionNominalIncompatible | semántico | reservado | futura comparación incompatible |
| BAS-7128 | MiembroNoVisible | semántico | activo | campo/método/tipo no visible |
| BAS-7129 | ResolucionMetodoAmbigua | semántico | reservado | ambigüedad futura multi-trait |
| BAS-7130 | RecursividadNominalPorValor | semántico | activo | ciclo por valor infinito |

## 21. Matriz de capacidades (resumen normativo)

Detalle en `propuesta-fase-08-s8.0-matriz-capacidades.md`. Resumen:

| Capacidad | Semántica | HIR/MIR | TW | VM | Nativo | WASM |
|---|---|---|---|---|---|---|
| Struct declaración/construcción/acceso/asignación | S8.1 | S8.1 | OBL | OBL | LIM | LIM/NO según subcapacidad |
| Enum unitario/payload posicional | S8.2 | S8.2 | OBL | OBL | LIM | NO inicial |
| Match exhaustivo patrones v1 | S8.3 | S8.3 | OBL | OBL | NO inicial | NO inicial |
| Métodos inherentes | S8.4a | S8.4a | OBL | OBL | NO inicial | NO inicial |
| Traits estáticos mínimos | S8.4b | S8.4b | OBL | OBL | NO | NO |
| Igualdad nominal | NO v1 (BAS-7126) | N/A | N/A | N/A | N/A | N/A |
| Genéricos nominales | NO v1 (BAS-7125) | N/A | N/A | N/A | N/A | N/A |

## 22. Aceptación y pruebas (contrato)

Detalle en `propuesta-fase-08-s8.0-matriz-aceptacion-pruebas.md`. Obligatorio por constructo:

- Positivos, negativos, límite, diferenciales TW↔VM.
- Verificadores HIR/MIR.
- No regresión G4 (33/33), G3, G2.2, G2.1, diferencial 145/145, workspace.
- Casos obligatorios D8-04/D8-05/D8-08: efectos secundarios de inicializadores, copia vs mutación, exhaustividad, binding, unreachable, visibilidad.

## 23. Plan S8.1–S8.5

Detalle en `PLAN-FASE-08-S8.0-S8.5.md`. Orden confirmado:

- S8.0 especificación/contrato (este documento).
- S8.1 structs nominales.
- S8.2 enums/ADTs.
- S8.3 pattern matching.
- S8.4a métodos inherentes, S8.4b traits mínimos.
- S8.5 consolidación/cierre.

Ninguna subetapa autorizada por S8.0.

## 24. Fuera de alcance v1

- Genéricos nominales, bounds, trait bounds, impl genéricas.
- Trait objects, dynamic dispatch, default methods, supertraits, associated types/constants.
- Guards, or-patterns, rangos de patrón, patrones de lista, `..` en struct pattern.
- Payload struct-like de variantes enum.
- Igualdad nominal automática.
- Referencias/borrowing/propiedad.
- Soporte universal nativo/WASM.
- Cambios en G4/M1–M3.

## 25. Criterio de salida S8.0 R2

S8.0 R2 queda **READY FOR FINAL HUMAN REVIEW** cuando:

1. No quedan decisiones fundamentales abiertas D8-01..D8-14.
2. Los 6 documentos son coherentes con esta fuente principal.
3. Sintaxis canónica única en ejemplos.
4. Diagnósticos clasificados y sin colisión conocida en catálogo actual.
5. Matriz de capacidades/aceptación cubre reglas normativas.
6. No hubo cambios de código/tests/DEVELOPMENT_STATE/ROADMAP_MASTER.
7. G4 permanece intacto.