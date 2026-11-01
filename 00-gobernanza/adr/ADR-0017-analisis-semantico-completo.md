# ADR-0017 — Análisis semántico completo de BASILISCO

**Estado:** Aprobado — Etapas 1–6 implementadas (infraestructura de símbodos, resolución de nombres/ámbitos, funciones/módulos, tipos/expresiones, diagnósticos/flujo, integración CLI)  
**Fecha:** 2026-11-01  
**Autor:** Arquitectura BASILISCO  
**Relacionado con:** ADR-0011 (tipos graduales), ADR-0008 (funciones), ADR-0010 (CLI), ADR-0012, ADR-0013, ADR-0014.

## 1. Contexto

BASILISCO 0.6.0 ya posee un pase semántico parcial en `prototipo/src/semantico.rs` (1444 líneas). Este pase analiza programas antes de ejecutar, registra funciones, valida ámbitos, resuelve módulos y produce diagnósticos (`ErrorEjecucion` con span). Sin embargo, no detecta todos los errores que el evaluador detecta en tiempo de ejecución (GAP-026 abierto). La inferencia por primera llamada (`tipos_parametros_inferidos`) fija tipos en la primera invocación. El comando `analizar` integra este pase pero no garantiza cobertura completa.

Capacidades ya existentes que el ADR debe preservar:
- Registro anticipado de funciones (recursión y referencias posteriores).
- Resolución de módulos y caché por proceso (`cargador.rs`).
- Funciones de primera clase, lambdas y closures con captura léxica.
- Funciones anidadas.
- Tipos graduales (anotados e inferidos).
- `Valor::Objeto`, `Valor::FechaHora` en runtime (no declarables aún por el usuario).
- Concurrencia básica (`iniciar_tarea`/`esperar_tarea`).

## 2. Problema

El análisis semántico actual está acoplado al evaluador y no es completo: no cubre todos los errores de tipo en expresiones complejas, accesos a símbolos privados en casos de borde, ni validaciones de flujo de control. No existe una arquitectura formal que defina responsabilidades, interfaces y etapas de evolución. Sin ella, cada mejora corre el riesgo de duplicar lógica del evaluador o de romper programas existentes.

## 3. Decisión

Introducir un pase semántico independiente y versionado, separado del evaluador, que:

1. Opere sobre el AST producido por `parser.rs`.
2. Produzca un conjunto de diagnósticos sin ejecutar el programa.
3. Mantenga una tabla de símbolos externa al AST (no anotaciones en el AST inicialmente).
4. Preserve todas las validaciones de seguridad del evaluador (no eliminarlas).
5. El análisis estático se ejecuta mediante `bsk analizar`; `bsk run` no ejecuta automáticamente el análisis estático y conserva las validaciones dinámicas del evaluador.
6. Sea incremental: etapas pequeñas y revisables.

Decisión sobre representación: **mantener el AST original y asociar información semántica externamente** (tablas de símbolos + scopes). No introducir una IR ni anotaciones en el AST en esta fase. Esto minimiza cambios en parser/evaluador y permite evolución gradual.

## 4. Alternativas consideradas

- **Anotar el AST con IDs de símbolo**: descartado por ahora porque requiere modificar el parser y todos los consumidores (evaluador, formateador). Se evaluará en etapa posterior.
- **Representación semántica intermedia (HIR)**: descartado para la fase inicial; se reserva para cuando se necesite optimización o backends (IR/VM, GAP-027).
- **Acoplar análisis al evaluador**: estado actual; se mantiene como fallback pero se reduce progresivamente.

## 5. Arquitectura propuesta

```
Código fuente
   |
 Lexer
   |
Parser
   |
AST
   |
Analizador semántico  (nuevo módulo, externo)
   |--- Tabla de símbolos
   |--- Gestión de ámbitos
   |--- Resolución de nombres
   |--- Análisis de tipos
   |--- Análisis de funciones
   |--- Diagnósticos
   |
Diagnósticos + AST validado
   |
Evaluador (sin cambios de contrato)
```

Componentes propuestos (en un solo módulo inicial, `semantico.rs`, subdividido internamente):

- `TablaSimbolos`: mapa de símbolos por id.
- `GestorAmbitos`: pila de ámbitos con resolución léxica.
- `AnalizadorSemantico`: orquestador.
- `Diagnosticador`: acumulador de diagnósticos con span.

## 6. Modelo de símbolos

Símbolo base:

```
struct Simbodo { // nombre tentativo
    id: SimbodoId (u32 estable por programa),
    nombre: String,
    ambito: AmbitoId,
    tipo: TipoSem,
    mutable: bool,
    ...
}
```

Categorías:
- Variable (local/global/módulo).
- Función (nombrada, lambda, clausura).
- Parámetro.
- Módulo importado.
- Constante/builtin.

Reglas de visibilidad: léxica estándar. Sombreado permitido en ámbitos anidados (preserva comportamiento actual). Declaraciones duplicadas en el mismo ámbito → error estático.

Resolución de nombres: búsqueda desde el ámbito actual hacia arriba; globales y módulos en la raíz.

## 7. Gestión de ámbitos

- Global: variables y funciones de nivel superior.
- Función: parámetros + locales.
- Bloque: `{ ... }` introduce ámbito anidado.
- Closure: captura léxica; el análisis registra variables capturadas pero no las duplica.
- Recursividad: registro anticipado de firmas de funciones antes de analizar cuerpos.
- Mutua recursividad: soportada por registro anticipado.
- Sombreado: permitido, documentado.

## 8. Análisis de funciones

- Firmas y aridad validadas en llamadas (estático cuando los argumentos son literales/conocidos; dinámico para valores runtime).
- Parámetros con tipos anotados o inferidos.
- Retorno: respetar E1010 (llamada como sentencia → Ok(None); como expresión → error si no hay valor). El análisis no debe redefinir esto.
- Funciones sin retorno: válidas como sentencia.
- Funciones de primera clase: tipo `función` compatible.
- Lambdas/closures: analizar captura y tipos.
- Funciones anidadas: ámbito léxico.
- Llamadas indirectas: validar que el callable es función.
- Referencias posteriores: por registro anticipado.

## 9. Sistema de tipos

Integrar gradualmente (ADR-0011). Tipos actuales: entero, decimal, real, lógico, texto, arreglo, lista, nulo, archivo, módulo, función, clausura, objeto, fechahora.

- Primitivos y anotaciones existentes.
- Inferencia por primera llamada (comportamiento actual, documentado como frágil pero preservado).
- Compatibilidad y conversiones existentes (implícitas sin pérdida).
- `Valor::Objeto` y `Valor::FechaHora`: existen en runtime pero NO son declarables por el usuario todavía. El análisis no debe asumir anotaciones de tipo objeto ni fechahora hasta que se diseñen en un ADR específico.

Extensibilidad futura (sin rediseño): genéricos, uniones, algebraicos → etapas posteriores.

## 10. Validaciones estáticas (alcance inicial)

Estático (antes de ejecutar):
- Variables no declaradas.
- Funciones inexistentes.
- Declaraciones duplicadas en mismo ámbito.
- Aridad incorrecta en llamadas directas.
- Tipos incompatibles en operaciones binarias cuando ambos son literales/anotados.
- Retornos fuera de función.
- Llamadas indirectas con callable no-función (cuando inferible).
- Errores de resolución de módulos (rutas, ciclos).
- Código inalcanzable (opcional, fase posterior).

Dinámico (solo en ejecución, no prometer estático):
- Valores dependientes de entrada de usuario/archivo.
- Tipos inferidos por primera llamada.
- Resultados de operaciones con overflow.
- E/S y permisos.

## 11. Diagnósticos

Estructura:

```
struct Diagnostico {
    codigo: String,        // E1xxx
    categoria: Categoria,  // Semántico, Tipo, Nombre, Módulo, ...
    mensaje: String,
    archivo: Option<PathBuf>,
    span: Span { inicio, fin, linea, columna },
    sugerencia: Option<String>,
}
```

Reutilizar `CausaError` y `ErrorEjecucion` existentes como base; ampliar con categoría y span final. Múltiples diagnósticos por ejecución de `analizar` (no detener en el primero). Ordenar por posición.

## 12. Integración con CLI

**`bsk analizar <archivo>`**
- Ejecuta el análisis estático.
- No ejecuta el programa.
- Acumula diagnósticos.
- Devuelve código de salida 1 cuando existen errores y 0 cuando el análisis termina correctamente.

**`bsk run <archivo>`**
- Ejecuta el programa.
- No ejecuta automáticamente el análisis estático.
- Conserva las verificaciones dinámicas del evaluador.

**`--sin-analisis`**
- No está implementado.
- No existe actualmente como opción de CLI.
- Se registra como funcionalidad pendiente, no como capacidad disponible.

## 13. Compatibilidad con el evaluador

- El evaluador conserva todas sus verificaciones de seguridad.
- El análisis semántico no duplica la ejecución.
- Información fluye: diagnósticos + tabla de símbolos (opcional) al evaluador; el evaluador no depende del análisis para funcionar.
- No modificar closures ni captura de entorno léxico.
- No eliminar validaciones existentes sin pruebas de equivalencia.

## 14. Plan de implementación (etapas)

Etapa 1 — Consolidación del pase actual (sin cambios de comportamiento):
- Objetivo: documentar y testear el semantico.rs existente.
- Archivos: `semantico.rs`, tests `semantico*.rs`.
- Riesgo: bajo. Criterio: 100% tests existentes pasan.

Etapa 2 — Tabla de símbolos externa y IDs estables:
- Objetivo: extraer tabla de símbolos independiente del AST.
- Archivos: nuevo `simbolos.rs` (o sección en semantico.rs).
- Riesgo: medio. Criterio: resolución equivalente a la actual.

Etapa 3 — Resolución de nombres completa (variables, funciones, módulos):
- Objetivo: cubrir casos de borde (sombreado, importaciones, privados).
- Riesgo: medio. Criterio: pruebas negativas nuevas pasan.

Etapa 4 — Tipos estáticos ampliados:
- Objetivo: validar tipos en expresiones complejas cuando anotados.
- Riesgo: alto. Criterio: sin regresiones en inferencia por primera llamada.

Etapa 5 — Flujo de control y alcanzabilidad (opcional):
- Objetivo: detectar código inalcanzable y retornos faltantes estáticamente.
- Riesgo: alto. Criterio: documentado y acotado.

Etapa 6 — CLI `analizar`/`run` integración final:
- Objetivo: código de salida y diagnósticos estables.
- Riesgo: bajo.
- Resultado: integración completada según la sección 12 (dos caminos independientes: análisis estático y ejecución dinámica).

## 15. Consecuencias

Positivas: base sólida para evolución, mejor DX, diagnósticos claros, compatibilidad con implementación incremental.

Negativas: trabajo adicional, riesgo de divergencia entre diagnósticos estáticos y errores detectados en runtime al mantener dos caminos independientes (analizar vs run), complejidad de mantener ambos.

Limitaciones pendientes:
- Inferencia por primera llamada frágil.
- Sin análisis de código inalcanzable.
- Sin garantía completa de retorno estático.
- Diagnósticos sin span final ni sugerencias.
- Sin genéricos, uniones ni tipos algebraicos.
- Sin opción `--sin-analisis`.

Riesgos: inferencia frágil por primera llamada puede generar falsos positivos; cambios en tipos graduales requieren coordinación con ADR-0011.

## 16. Criterios de aceptación del ADR

- Documento creado en `00-gobernanza/adr/ADR-0017-analisis-semantico-completo.md`.
- Numeración correcta (0017, siguiente disponible).
- No se modifica código fuente.
- No se introducen dependencias.
- Versión del lenguaje sin cambios.

## 17. Informe de auditoría

Archivos inspeccionados: `lexer.rs`, `parser.rs`, `semantico.rs`, `evaluador.rs`, `valor.rs`, `cargador.rs`, `cli.rs`, tests `semantico*.rs`, `modulos_e2e.rs`, `conformidad*.rs`.

Estado semántico actual: pase parcial en `semantico.rs` con registro de funciones, ámbitos, resolución de módulos y validación de tipos anotados. No cubre todos los errores de runtime.

Capacidades ya existentes: registro anticipado, recursión, lambdas/closures, módulos, tipos graduales, diagnósticos con span.

Brechas reales: GAP-026 (análisis semántico estático incompleto), inferencia por primera llamada frágil, divergencia analizar/run en casos de borde.

Decisiones arquitectónicas: tabla de símbolos externa, AST sin anotaciones en fase inicial, análisis opcional en run, obligatorio en analizar, incremental.

Alternativas descartadas: anotación del AST, HIR, acoplamiento al evaluador como única vía.

Etapas propuestas: 6 etapas listadas en sección 14.

Riesgos pendientes: sincronización análisis/evaluador, falsos positivos por inferencia, evolución de tipos graduales.
EOF
echo "created"