# Catálogo de errores y diagnósticos

## Objetivo
Los errores son parte de la interfaz pública del lenguaje. Deben ser consistentes, localizables, comprobables y aptos para IDE/CLI.

## Estructura recomendada
Cada diagnóstico debe contener:
- `codigo`: identificador estable, por ejemplo `NXE-TYPE-001`;
- `categoria`: léxico, sintáctico, semántico, tipo, ejecución, recurso o herramienta;
- `mensaje`: explicación breve en español;
- `span_primario`: archivo, línea, columna inicial y final;
- `spans_secundarios`: declaraciones o expresiones relacionadas;
- `ayuda`: acción posible, si es segura;
- `documentacion`: identificador de referencia opcional;
- `severidad`: error, advertencia o nota.

## Familias propuestas
- `NXE-LEX-*`: caracteres y literales mal formados.
- `NXE-PARSE-*`: estructura sintáctica inválida.
- `NXE-NAME-*`: nombres no declarados o duplicados.
- `NXE-TYPE-*`: incompatibilidades de tipo.
- `NXE-ARITH-*`: overflow, división por cero y operaciones numéricas inválidas.
- `NXE-RUNTIME-*`: fallos definidos de ejecución.
- `NXE-RESOURCE-*`: límites de recursos.
- `NXW-*`: advertencias no fatales.

Los prefijos son una propuesta; deben aprobarse antes de fijarlos como interfaz pública.

## Reglas
1. No incluir stack traces internos en el mensaje normal para usuarios.
2. La versión legible puede cambiar; el código debe mantenerse estable.
3. No sugerir arreglos que cambien el significado sin indicarlo.
4. Distinguir el error original de los errores derivados para evitar cascadas.
5. Los diagnósticos deben poder emitirse en texto y en formato máquina.
6. Los snapshots de pruebas deben comprobar código y ubicación; no depender únicamente de la redacción.
7. El color no puede ser el único indicador de severidad.
8. El parser debe evitar mensajes múltiples engañosos tras un único fallo raíz.

## Pendiente
Definir columnas en bytes, puntos de código o unidades UTF-16; comportamiento con tabulaciones; normalización de identificadores; estrategia de recuperación sintáctica y códigos definitivos.


## Sincronización con código (prototipo)

Este catálogo se sincroniza con `prototipo/src/evaluador.rs::CausaError` (fuente de verdad en implementación).

```rust
pub enum CausaError {
    /// Motivo aritmético o de tipos del sistema numérico (ADR-0005).
    Aritmetica(MotivoAritmetico),
    /// Referencia o asignación a un nombre no declarado (ADR-0006).
    VariableNoDefinida(String),
    /// `sea nombre` repetido en el mismo ámbito (ADR-0006).
    DeclaracionDuplicada(String),
    /// Asignación con un tipo distinto al declarado y sin conversión implícita.
    AsignacionIncompatible {
        esperado: &'static str,
        obtenido: &'static str,
    }
```

**Regla:** Cualquier nueva causa en el evaluador debe reflejarse aquí con código estable, mensaje, ubicación y acción sugerida.
