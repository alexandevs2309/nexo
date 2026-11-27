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

## Códigos implementados en el prototipo (E1xxx)
El prototipo asigna códigos estables `E1xxx` a `CausaError`:

| Código | Variante |
|---|---|
| E1001 | Aritmetica |
| E1002 | VariableNoDefinida |
| E1003 | DeclaracionDuplicada |
| E1004 | AsignacionIncompatible |
| E1005 | CondicionNoLogica |
| E1006 | OperandoNoLogico |
| E1007 | TiposIncompatibles |
| E1008 | LimiteIteraciones |
| E1009 | FirmaInvalida |
| E1010 | RetornoAusente |
| E1011 | TipoDeRetornoIncompatible |
| E1012 | RetornarFueraDeBloque |
| E1013 | TipoParametroIncompatible |
| E1014 | RetornoIncompatibleConAnotacion |
| E1015 | SenalRomper |
| E1016 | SenalContinuar |
| E1017 | IteracionSobreNoColeccion |
| E1018 | RangoTipoInvalido |
| E1019 | ModificacionDuranteIteracion |
| E1020 | NombreReservado |
| E1021 | EntradaInvalida |
| E1022 | ArchivoNoEncontrado |
| E1023 | PermisosDenegados |
| E1024 | ErrorDeArchivo |
| E1025 | ArchivoYaCerrado |
| E1026 | IndiceTextoFueraDeRango |
| E1027 | ConversionTextoInvalida |
| E1028 | SeparadorVacio |
| E1029 | ReasignacionInmutable |
| E1030 | ModuloNoEncontrado |
| E1031 | CicloDeImportacion |
| E1032 | SimbodoPrivado |
| E1033 | SimbodoNoExportado |
| E1034 | ImportacionDuplicada |
| E1035 | ExtensionNoSoportada |
| E1036 | RutaNoPermitida |
| E1037 | SentenciaNoPermitidaEnModulo |
| E1038 | ImportacionesNoDisponibles |
| E1040 | FFIInvalido (legado/genérico; Fase 11 desagrega en E1070-E1082) |
| E1070 | FfiBibliotecaNoEncontrada |
| E1071 | FfiBibliotecaNoCargable |
| E1072 | FfiSimboloNoEncontrado |
| E1073 | FfiIdBibliotecaInvalido |
| E1074 | FfiBibliotecaDescargada |
| E1075 | FfiFirmaInvalida |
| E1076 | FfiAridadIncompatible |
| E1077 | FfiTipoArgumentoIncompatible |
| E1078 | FfiConversionFueraDeRango |
| E1079 | FfiResultadoIncompatible |
| E1080 | FfiErrorInvocacion |
| E1081 | FfiEstadoRecursoInvalido |
| E1082 | FfiMemoria |

Los códigos se emiten en el mensaje de `ErrorEjecucion` como `[E1xxx]`.

## Pendiente
Definir columnas en bytes, puntos de código o unidades UTF-16; comportamiento con tabulaciones; normalización de identificadores; estrategia de recuperación sintáctica y códigos definitivos.


## Errores de paquetes y dependencias

La resolución de paquetes ocurre antes de ejecutar el programa y no produce
códigos `E1xxx`. El CLI devuelve código de salida `3` y un mensaje en `stderr`:

- manifiesto ausente, ilegible o inválido;
- nombre o versión de paquete inválidos;
- dependencia duplicada o no encontrada;
- versión solicitada distinta de la declarada por el paquete;
- error al escribir o leer `basilisco.lock`.

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
    /// Sentencia no permitida en el nivel superior de un módulo (ADR-0017 D4).
    SentenciaNoPermitidaEnModulo { sentencia: String },
    /// Importaciones no disponibles (sin cargador configurado) (ADR-0017 D12).
    ImportacionesNoDisponibles,
    /// Error al cargar o llamar una biblioteca dinámica (FFI básico).
    FFIInvalido { detalle: String },
}
```

**Regla:** Cualquier nueva causa en el evaluador debe reflejarse aquí con código estable, mensaje, ubicación y acción sugerida.
