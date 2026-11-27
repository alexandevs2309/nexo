# FFI / ABI — Fase 11

> **Estado:** IMPLEMENTED + VERIFIED (subconjunto ABI C, Fase 11). READY FOR HUMAN REVIEW en alcance definido.
> **Motores soportados:** Tree-Walk y VM (paridad verificada en subconjunto). Backend nativo/WASM: rechazo explícito BAS-7124.

## 1. Modelo de interoperabilidad

BASILISCO expone un puente FFI C mediante tres primitivas:
- `cargar_ffi(ruta: texto) -> entero` — carga biblioteca dinámica, devuelve id.
- `llamar_ffi(id: entero, simbolo: texto, args: lista, tipo_retorno?: texto) -> valor` — invoca símbolo.
- `descargar_ffi(id: entero) -> nulo` — descarga biblioteca por id.

Implementación: `prototipo/src/ffi.rs` (`libloading` para carga, `libffi` para llamadas). Errores tipados `CausaFfi` integrados en `CausaError::FFIInvalido` (TW) y `ErrorVm::FfiInvalido` (VM).

## 2. Contrato ABI

- **ABI:** C ABI de la plataforma (`extern "C"` en el lado C). Convención de llamada: la provista por libffi para la plataforma/toolchain.
- **Plataforma verificada:** Linux x86_64 (entorno CI/local de desarrollo), compilador `cc` disponible.
- **Bibliotecas:** dinámicas (`.so` Linux, `.dll` Windows, `.dylib` macOS por convención de nombre; verificado en Linux).
- **Símbolos:** resueltos por nombre; el nombre no debe contener byte NUL.
- **Duración:** la biblioteca se mantiene viva por `Arc<Library>` durante la invocación aunque otro hilo la descargue. Tras `descargar_ffi`, el id se invalida; llamadas posteriores producen error.

## 3. Tipos y conversiones

| Valor BASILISCO | Tipo C ABI | Notas |
|---|---|---|
| `entero` | `int64_t` / `long long` | 64 bits con signo |
| `real` | `double` | IEEE 754 64-bit |
| `decimal` | `double` | Escalado BASILISCO /10^9 antes de pasar; retorno `double` reescalado a `decimal`, error si no finito o desborda i128 |
| `texto` | `const char*` | UTF-8 NUL-terminated. Argumento: `CString` propiedad de BASILISCO durante la llamada; callee no debe conservar/liberar. Retorno: BASILISCO copia contenido, no libera puntero; nulo => error. |

| tipo_retorno | Tipo C ABI | Resultado BASILISCO |
|---|---|---|
| `entero` (default) | `int64_t` | `entero` |
| `logico` | `int64_t` | `lógico` (`!= 0`) |
| `real` | `double` | `real` |
| `decimal` | `double` | `decimal` o error de rango |
| `texto` | `const char*` | `texto` (copia) o error si nulo |
| `nulo`/`vacío` | `void` | `nulo` |

No hay conversión silenciosa entre tipos incompatibles. Tipos no admitidos como argumento (`lógico`, colecciones, objetos, etc.) producen `E1077`.

## 4. Firmas y validación

- La firma ABI se expresa por los valores reales pasados y `tipo_retorno`. libffi construye el CIF antes de la llamada.
- No se valida la firma C real del símbolo (limitación ABI C dinámica). Una firma incompatible puede causar UB nativo; el contrato exige que el programa BASILISCO coincida con la firma C.
- `tipo_retorno` desconocido => `E1075`.
- El identificador de biblioteca inválido/descargado => `E1073`/`E1074`.

## 5. Memoria y seguridad

- **Argumentos texto:** `CString` vivo durante toda la invocación. El callee no debe retener el puntero.
- **Retorno texto:** BASILISCO lee con `CStr::from_ptr` y copia a `String`. No libera el puntero. Contrato: puntero válido, NUL-terminated, legible durante la conversión. Punteros retornados por `malloc` no liberados por callee son fuga de responsabilidad del callee (límite documentado).
- **Bibliotecas:** `Arc<Library>` evita descarga efectiva durante llamada en curso. `descargar_ffi` elimina id del registro.
- **Pánico Rust:** llamadas libffi envueltas con `catch_unwind`; pánico no controlado => `E1080`. No impide abort/segfault nativo.
- **Límites de seguridad:** BASILISCO no puede validar punteros arbitrarios devueltos por C ni impedir UB/corrupción en código nativo. FFI convencional no es sandbox. Código nativo no confiable requiere aislamiento por proceso (fuera de alcance Fase 11).

## 6. Errores y diagnósticos

| Código | Causa | Descripción |
|---|---|---|
| E1070 | BibliotecaNoEncontrada | Ruta inexistente/no encontrable |
| E1071 | BibliotecaNoCargable | Existe pero no cargable/incompatible |
| E1072 | SimboloNoEncontrado | Símbolo no resuelto |
| E1073 | IdBibliotecaInvalido | Id no válido en registro |
| E1074 | BibliotecaDescargada | Biblioteca ya descargada / id no presente |
| E1075 | FirmaInvalida | Tipo de retorno/firma inválida |
| E1076 | AridadIncompatible | Reservado (no validable sin metadata C) |
| E1077 | TipoArgumentoIncompatible | Tipo de argumento no admitido/con NUL interno |
| E1078 | ConversionFueraDeRango | Decimal no finito/desbordado |
| E1079 | ResultadoIncompatible | Retorno incompatible (p.ej. texto nulo) |
| E1080 | ErrorInvocacion | Error de invocación/pánico capturado |
| E1081 | EstadoRecursoInvalido | Estado de recurso inválido |
| E1082 | Memoria | Error de memoria controlable |

Normalización paridad TW↔VM: categoría `FfiInvalido`, subcausa = código, detalle normaliza ids de runtime.

## 7. Integración por motor

| Motor | Estado | Evidencia |
|---|---|---|
| Tree-Walk | SUPPORTED | `prototipo/tests/ffi.rs` (E2E real con cc/libffi/libloading) |
| VM | SUPPORTED | Mismos casos E2E y paridad TW↔VM |
| Backend nativo | UNSUPPORTED | Rechazo explícito `BAS-7124` al detectar `cargar_ffi/llamar_ffi/descargar_ffi` |
| WASM | UNSUPPORTED | Rechazo explícito `BAS-7124` en backend entero/WASM; carga dinámica nativa no aplicable |

## 8. Matriz de compatibilidad

| Dimensión | Valor | Clasificación |
|---|---|---|
| ABI | C ABI plataforma, libffi | SUPPORTED (verificado Linux x86_64) |
| Plataformas | Linux x86_64 verificado; Windows/macOS por diseño libloading/libffi | Linux SUPPORTED, otros UNVERIFIED |
| Tipos args | entero, real, decimal, texto | SUPPORTED |
| Tipos ret | entero, logico, real, decimal, texto, nulo/vacío | SUPPORTED |
| Carga/descarga/símbolos | cargar/descargar/resolver | SUPPORTED |
| Texto NUL interno | rechazado | SUPPORTED |
| Punteros opacos | no expuestos | UNSUPPORTED |
| Structs C | no | UNSUPPORTED |
| Callbacks C→BASILISCO | no | UNSUPPORTED |
| Variádicos C | no | UNSUPPORTED |
| Buffers/memoria raw | no | UNSUPPORTED |
| Aislamiento de código nativo | no | UNSUPPORTED (requiere arquitectura proceso aislado) |

## 9. Limitaciones

- No validación de firma C real del símbolo (riesgo UB si el programa no coincide con ABI C).
- No callbacks, structs, variádicos, punteros opacos, buffers raw.
- Retorno texto: copia sin liberar; puntero nulo rechazado; puntero inválido/UTF-8 no válido puede causar comportamiento definido por contrato de CStr/to_string_lossy (se copia con reemplazo de UTF-8 inválido).
- `descargar_ffi` invalida id; no garantiza que código C haya liberado sus propios recursos.
- FFI global por proceso (registro estático), no aislado por contexto de ejecución BASILISCO.
- No sandbox: crashes/UB nativos pueden terminar el proceso.
