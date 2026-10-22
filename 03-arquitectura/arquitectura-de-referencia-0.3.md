# Arquitectura de referencia Nexo 0.3

## Objetivo
Definir límites de componentes sin comprometer prematuramente un backend definitivo. La arquitectura debe permitir evolucionar el prototipo sin confundir frontend, semántica, ejecución y herramientas.

## Flujo lógico
1. Entrada de bytes y decodificación de texto.
2. Lexer: tokens y spans de origen.
3. Parser: AST y recuperación controlada de errores.
4. Análisis semántico: nombres, ámbitos, tipos y restricciones.
5. Representación intermedia opcional cuando el caso de uso lo justifique.
6. Ejecución por intérprete, VM o backend seleccionado mediante ADR.
7. Biblioteca estándar y adaptadores de plataforma.
8. CLI y herramientas de desarrollo.
9. Pruebas de conformidad y diagnósticos compartidos.

## Contratos de componentes
| Componente | Entrada | Salida | Obligación |
|---|---|---|---|
| Decodificador | bytes | texto o error | UTF-8/encoding definido y posición |
| Lexer | texto | tokens + spans + errores | no perder posiciones de origen |
| Parser | tokens | AST o diagnósticos | no ejecutar código durante parseo |
| Analizador | AST | símbolos/tipos/diagnósticos | reglas explícitas y deterministas |
| Evaluador/runtime | programa analizado | valor/salida/error | errores controlados y límites de recursos |
| Diagnósticos | error estructurado | texto/formato máquina | código estable, localización |
| CLI | argumentos/proyecto | resultado y código de salida | comandos y salida reproducibles |
| Test runner | casos | informe | distinguir pass/fail/skip/error |

## Decisiones abiertas
- AST interpretado, bytecode VM, compilación nativa o arquitectura híbrida.
- Tipado estático, dinámico o mixto.
- Modelo de memoria y propiedad.
- Formato de módulos y representación intermedia.
- API estable entre frontend, runtime y herramientas.

Cada alternativa debe evaluarse mediante criterios ponderados: corrección, rendimiento medido, portabilidad, complejidad de implementación, diagnósticos, seguridad, depuración, interoperabilidad y coste de mantenimiento.

## Reglas de diseño
- Evitar que el parser ejecute efectos.
- Separar errores de usuario de defectos internos.
- Mantener spans hasta el diagnóstico final.
- Evitar que herramientas externas dependan de estructuras internas no versionadas.
- No diseñar una ABI pública antes de estabilizar tipos, memoria y errores.
- Medir antes de optimizar; cualquier optimización debe preservar la suite de conformidad.
