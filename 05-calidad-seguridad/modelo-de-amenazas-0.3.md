# Modelo de amenazas de BASILISCO 0.3 (histórico «Nexo»)

## Activos
- archivos del usuario y credenciales accesibles desde programas;
- integridad del ejecutable, runtime, paquetes y actualizaciones;
- secretos y datos de proyectos;
- disponibilidad de CPU, memoria, disco y red;
- confianza en diagnósticos, documentación y resultados de ejecución.

## Límites de confianza
- fuente no confiable;
- paquetes y scripts de terceros;
- extensiones/plugins del IDE;
- bibliotecas nativas llamadas mediante FFI;
- compiladores, runners y servidores de CI;
- canal de distribución y actualizador.

## Amenazas principales
| Amenaza | Control mínimo recomendado | Evidencia necesaria |
|---|---|---|
| Entrada maliciosa provoca panic/caída | fuzzing, errores controlados, límites | corpus y CI |
| Bucle o recursión consume recursos | límites configurables y cancelación | pruebas de agotamiento |
| Paquete comprometido | hashes, firmas, revisión y procedencia | verificación en instalación |
| Script lee archivos inesperados | permisos explícitos/sandbox cuando aplique | pruebas de aislamiento |
| Dependencia ejecuta scripts al instalar | instalación sin ejecución por defecto | política y tests |
| FFI corrompe memoria | contratos ABI/ownership y pruebas nativas | auditorías y sanitizadores |
| Actualización manipulada | firma, TLS y rollback seguro | pruebas de actualización |
| Datos sensibles en logs | redacción y política de telemetría | pruebas de filtrado |
| Ataque a servidor de paquetes | permisos mínimos, rate limiting y backups | auditoría operativa |

## Principios
- Menor privilegio.
- Denegar por defecto cuando se ejecuten dependencias o código no confiable.
- No prometer sandbox seguro hasta validarlo por plataforma.
- Separar la ejecución de código del análisis estático.
- No habilitar red, shell o acceso a archivos implícitamente en herramientas que procesen proyectos de terceros.
- La telemetría debe ser transparente, minimizada y desactivable si se introduce.
- La seguridad de la cadena de suministro incluye desarrollo, build, publicación y actualización.

## Riesgo residual
El lenguaje no puede garantizar seguridad total solo por su sintaxis. Las bibliotecas, FFI, runtime, permisos del sistema operativo, paquetes y prácticas del usuario forman parte del modelo.
