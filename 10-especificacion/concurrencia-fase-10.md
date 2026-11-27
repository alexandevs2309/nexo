# Concurrencia — Fase 10

**Estado:** READY FOR HUMAN REVIEW (implementación completada, pendiente aprobación humana)  
**Alcance:** tareas (`iniciar_tarea`, `esperar_tarea`) y canales (`crear_canal`, `enviar`, `recibir`, `cerrar_canal`) en motores Tree-Walk y VM.

## 1. Modelo de ejecución

- El modelo es de concurrencia por hilos del sistema operativo, con primitivas de tarea y canal expuestas por stdlib.
- Cada ejecución top-level usa un contexto de ejecución aislado (registros de canales y tareas) instalado por hilo. Las tareas hijas heredan el contexto del padre mediante registros compartidos (`Arc<Mutex<...>>`) instalados al iniciar el hilo.
- El estado de canales/tareas pertenece al contexto de ejecución, no es global de proceso fuera del contexto actual. Los contextos gestionados se aíslan; el fallback lazy preserva compatibilidad pero las entradas top-level normales instalan contexto fresco.
- No se introduce async/await, runtime nuevo, actores, distribución ni nuevas primitivas de sincronización generales.

## 2. Tareas

### API

- `iniciar_tarea("nombre") -> entero`: inicia la función de nombre `nombre` (sin argumentos) en un hilo nuevo. Devuelve un identificador entero de tarea dentro del contexto actual.
- `esperar_tarea(id) -> valor`: espera la finalización de la tarea `id` y devuelve su valor de retorno.

### Ciclo de vida

1. **Creada/iniciada**: `iniciar_tarea` registra un handle y devuelve id.
2. **En ejecución**: el hilo ejecuta la función en un motor con contexto compartido de canales/tareas.
3. **Finalizada con valor**: `esperar_tarea` devuelve el valor una vez.
4. **Finalizada con error/pánico**: `esperar_tarea` propaga error tipado.
5. **Consumida**: tras un `esperar_tarea` exitoso o fallido que consume el handle, la tarea queda esperada.

### Semántica de errores

Errores normalizados (TW↔VM, categoría `NoSoportado` con subcausa de concurrencia):

| Caso | Subcausa | Detalle canónico |
|---|---:|---|
| Función no encontrada en `iniciar_tarea` | `tarea_funcion_no_encontrada` | `tarea: función no encontrada: <nombre>` |
| Id inexistente en `esperar_tarea` | `tarea_inexistente` | `tarea inexistente: <id>` |
| Doble espera / handle ya consumido | `tarea_ya_esperada` | `tarea ya esperada: <id>` |
| Error/pánico de tarea | `tarea_fallida` | `tarea falló` (detalle interno colapsado para paridad) |

### Reglas

- Esperar tarea inexistente: error `tarea_inexistente`.
- Esperar tarea finalizada: el primer `esperar_tarea` consume el resultado; el segundo produce `tarea_ya_esperada`.
- Esperar tarea que falla/paniquea: error `tarea_fallida`.
- Tareas anidadas: soportadas dentro del mismo contexto compartido.
- El id de tarea es local al contexto de ejecución actual.

## 3. Canales

### API

- `crear_canal() -> entero`: crea un canal FIFO y devuelve su id.
- `enviar(id, valor) -> nulo`: encola un valor en el canal.
- `recibir(id) -> valor`: desencola el siguiente valor; bloquea si el canal está vacío y no cerrado.
- `cerrar_canal(id) -> nulo`: cierra el canal.

### Semántica

- **Orden**: FIFO por canal. Los mensajes se reciben en el orden en que fueron encolados.
- **Capacidad**: canal no acotado en esta fase (buffer en memoria). No hay bloqueo por capacidad.
- **Cierre**: `cerrar_canal` marca el canal como cerrado e notifica a esperas (`notify_all`). Es idempotente.
- **Enviar tras cierre**: error `canal_cerrado`.
- **Recibir con buffer no vacío**: devuelve el valor más antiguo, incluso si el canal está cerrado.
- **Recibir con buffer vacío y canal cerrado**: error `canal_cerrado_vacio`.
- **Recibir con buffer vacío y canal abierto**: bloquea hasta que llegue un valor o el canal se cierre.
- **IDs inválidos/inexistentes**: error `canal_inexistente`.

### Errores

| Caso | Subcausa | Detalle canónico |
|---|---:|---|
| Canal inexistente | `canal_inexistente` | `canal inexistente: <id>` |
| Envío a canal cerrado | `canal_cerrado` | `canal cerrado: <id>` |
| Recepción en canal cerrado y vacío | `canal_cerrado_vacio` | `canal cerrado y vacío: <id>` |

## 4. Aislamiento y sincronización

- Registros de tareas/canales son por contexto, protegidos por `Mutex`; los canales usan `Condvar` para espera.
- `enviar`/`recibir`/`cerrar_canal` no mantienen el lock del registro mientras bloquean; toman el `Arc` del canal y liberan el registro antes de esperar.
- `recibir` espera en bucle sobre la condición (buffer vacío y no cerrado), evitando wakeups espurios.
- Los hilos de tarea instalan los registros del contexto padre antes de ejecutar.
- La limpieza de contextos gestionados se realiza por guarda RAII.

## 5. Paridad TW↔VM

- El subconjunto soportado tiene paridad observable TW↔VM: creación/espera de tareas, errores de tareas, creación/envío/recepción/cierre de canales, FIFO, drenado antes de cierre, estados inválidos y tareas anidadas.
- Los errores de concurrencia se normalizan a la misma categoría/subcausa en `paridad.rs`.

## 6. Cancelación y timeout

- **Fuera del contrato obligatorio de Fase 10.** No se implementa cancelación cooperativa/forzosa ni timeout.
- Razón: la arquitectura actual no tiene puntos de cancelación seguros en loops/bloqueos del motor; una cancelación falsa o timeout que abandone esperas puede dejar hilos/estado inconsistente.
- Cualquier futura incorporación requiere diseño específico, puntos de cancelación, semántica de recursos/canales/tareas hijas y autorización de contrato.

## 7. Fin de ejecución principal con tareas activas

Comportamiento definido y verificado (Fase 10, revisión focalizada):

- Al terminar `ejecutar_programa` (TW) o `ejecutar_indice` (VM), la guarda RAII restaura los thread-locals del hilo principal; **no** realiza join global ni cancelación de tareas.
- Los hilos de tareas conservan clones `Arc` de los registros de canales/tareas instalados por `instalar_registros`; los registros siguen válidos mientras exista al menos un `Arc`.
- Una tarea puede continuar ejecutándose y completando operaciones sobre esos registros tras el retorno de main (evidencia: `evaluador::tests::f10_tarea_sobrevive_fin_main_vm`, `f10_tarea_sobrevive_fin_main_tw`).
- Una tarea puede iniciar tareas/canales dentro del mismo contexto compartido mientras su hilo y los Arcs sigan vivos.
- **Limitación explícita:** no hay join global ni cancelación. Tareas bloqueadas (p. ej. `recibir` en canal abierto sin productores) o de larga duración pueden fugar hilos/recursos hasta el fin del proceso. No es acceso a contexto inválido, pero es fuga de vida útil si el programa no espera/cierra explícitamente.
- Cualquier cambio a join global, cancelación o timeout requiere diseño y autorización de contrato.

## 8. Limitaciones conocidas

- `iniciar_tarea` ejecuta funciones por nombre sin argumentos.
- Canales no acotados (sin backpressure por capacidad).
- No hay select/multiplexación, no hay broadcast, no hay cancelación/timeout.
- No hay join global al terminar main: tareas activas pueden sobrevivir y fugar hilos si quedan bloqueadas/no esperadas (ver sección 7).
- Los detalles internos de fallos de tarea se colapsan a `tarea falló` para paridad; el diagnóstico original sigue existiendo en la ejecución no diferencial.
- El planificador del SO puede afectar interleavings; las pruebas de determinismo cubren contratos observables, no orden de scheduling no especificado.
