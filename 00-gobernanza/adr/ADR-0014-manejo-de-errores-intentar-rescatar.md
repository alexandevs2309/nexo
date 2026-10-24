# ADR-0014 — Manejo de errores: `intentar`/`rescatar`

- Estado: **Aprobado** (2026-10-24)
- Fecha: 2026-10-24
- Responsable: proyecto BASILISCO
- Revisión prevista: al completar la implementación del incremento 8.
- Prerequisito: ADR-0011 aprobado (tipo `nulo`), ADR-0013 aprobado (errores de E/S).
- Relacionado: ADR-0005 §1.6 (errores sin valor por defecto), ADR-0007 §1.4/§1.5
  (bloques con ámbito), ADR-0008 §3.5 (causas de error tipadas).
- Resuelve brechas: GAP-009 (errores recuperables y propagación).
- Cubre decisiones de `decisiones-que-no-deben-inventarse.md`:
  "excepciones, Resultado u otro modelo de errores".
- `resultado<T>` **diferido** al incremento 9 (requiere genéricos).

---

## 1. Contexto y requisitos

El modelo de errores actual (ADR-0005 §1.6, ADR-0008 §3.5) produce `ErrorEjecucion`
con causa tipada y ubicación. Todo error aborta el programa. No hay forma de que el
código BASILISCO capture y maneje un error en tiempo de ejecución.

Esto hace imposible escribir código robusto: `leer_entero()` con entrada inválida,
`abrir()` sobre un archivo inexistente, o una división por cero terminan el programa
sin oportunidad de recuperación.

Requisitos:
- R1: Permitir capturar errores de ejecución sin abortar el programa.
- R2: Preservar el contrato de ADR-0005 §1.6: los errores siguen siendo resultados
  tipados con ubicación; `intentar`/`rescatar` los captura, no los silencia.
- R3: No introducir genéricos (`resultado<T>` diferido).
- R4: La variable de error en `rescatar` expone el mensaje como `texto`.
- R5: `retornar` dentro de `intentar` sigue siendo retorno de función, no error capturado.
- R6: `intentar`/`rescatar` captura **todos** los `ErrorEjecucion`.

---

## 2. Opciones consideradas

### O1 — Excepciones con propagación implícita (estilo Java/Python)
Los errores suben automáticamente por la pila de llamadas hasta encontrar un manejador.
- Ventajas: familiar; sin código adicional para propagar.
- Costes: el flujo de control es invisible; los errores pueden pasar silenciosamente
  por muchas capas; contradice el principio de ADR-0005 §1.6 (sin valores por defecto,
  sin silencios).
- **Descartada**.

### O2 — Tipo `resultado<T>` (Ok/Error)
Cada operación que puede fallar retorna `resultado<entero>`, `resultado<texto>`, etc.
- Ventajas: el tipo expresa el contrato de error; el compilador puede obligar al
  programador a manejarlo.
- Costes: requiere genéricos; differido explícitamente al incremento 9 por confirmación
  del propietario.
- **Diferida** al incremento 9.

### O3 — `intentar`/`rescatar` (propuesta seleccionada)
Bloque estructurado que captura errores de ejecución.
- Ventajas: sin genéricos; sin propagación implícita; flujo de control visible;
  coherente con ADR-0005 §1.6 (el error no desaparece; se maneja explícitamente).
- Costes: no expresa en el tipo de la función que puede fallar (eso es para `resultado<T>`).
- **Seleccionada**.

---

## 3. Decisión

### 3.1 Sintaxis

```
intentar {
    sentencias...
} rescatar e {
    sentencias_de_manejo...
}
```

- El `{` de `intentar` debe ir en la misma línea que `intentar` (ADR-0007 §1.6).
- El `rescatar` puede ir en la línea siguiente al `}` de `intentar` (mismo precedente
  que `sino` después de `}` en ADR-0007 §1.6 nota de implementación).
- El `{` de `rescatar` debe ir en la misma línea que `rescatar`.
- `e` (o cualquier identificador) es la variable de error disponible dentro del bloque
  `rescatar`. Es de tipo `texto` y contiene el mensaje del error.
- `rescatar` **sin** variable de error también es válido:
  ```
  intentar {
      ...
  } rescatar {
      mostrar "ocurrió un error"
  }
  ```

### 3.2 Semántica

1. Se ejecuta el bloque de `intentar`.
2. Si el bloque termina sin error → la variable de error no existe; el bloque `rescatar`
   **no** se ejecuta; la ejecución continúa después del `}` de `rescatar`.
3. Si ocurre un `ErrorEjecucion` dentro del bloque → se ejecuta el bloque `rescatar`
   con la variable de error enlazada al mensaje del error (`ErrorEjecucion.mensaje`).
4. Después del `rescatar` la ejecución continúa normalmente.
5. Si el bloque `rescatar` también produce un error, ese error **no** se captura por
   el mismo `intentar`/`rescatar`; sube al `intentar`/`rescatar` exterior o aborta.

### 3.3 Alcance de captura

`intentar` captura **todos** los `ErrorEjecucion` que se produzcan dentro del bloque,
incluyendo:
- Errores aritméticos (desbordamiento, división por cero, etc. — ADR-0005).
- Errores de variable (no definida, duplicada — ADR-0006).
- Errores de tipo (asignación incompatible — ADR-0006, ADR-0011).
- Errores de condición (condición no lógica — ADR-0007).
- Errores de función (firma inválida, retorno ausente — ADR-0008).
- Errores de E/S (archivo no encontrado, entrada inválida — ADR-0013).
- Errores de iteración (ADR-0012).
- Cualquier otro `ErrorEjecucion` futuro.

`intentar` **no** captura:
- **Errores sintácticos** (`ErrorSintactico`): ocurren antes de la ejecución; el
  programa no llega a ejecutarse.
- **Errores léxicos** (`ErrorLexico`): ídem.
- **`retornar`**: no es un error; es una señal de control de flujo. `retornar` dentro
  de `intentar` retorna de la función que lo contiene.
- **`romper`/`continuar`**: señales de ciclo; no son errores.
- El límite de iteraciones (`LimiteIteraciones`) **sí** se captura (es un
  `ErrorEjecucion`).

### 3.4 Variable de error

La variable de error en `rescatar e` es de tipo `texto`. Su valor es el campo
`mensaje` de `ErrorEjecucion` (el mismo texto que se imprimiría al abortar el programa).

```
intentar {
    sea x = 10 / 0
} rescatar e {
    mostrar "Error capturado: " + e
    # imprime: "Error capturado: división entre cero"
}
```

No se expone la causa tipada (`CausaError`) como valor del lenguaje; eso requeriría
tipos algebraicos que aún no están decididos. El programador puede discriminar por
contenido del texto si necesita distinción granular.

### 3.5 Valor de `intentar`/`rescatar` como bloque

`intentar`/`rescatar` **no produce valor** (coherente con ADR-0007 §1.6: los bloques no
producen valor). Si se necesita capturar el resultado, se declara una variable antes:

```
var resultado_op: texto = ""
intentar {
    resultado_op = texto(10 / 2)
} rescatar e {
    resultado_op = "error: " + e
}
mostrar resultado_op
```

### 3.6 Anidamiento

`intentar`/`rescatar` puede anidarse. El error es capturado por el `intentar` más
cercano que lo contiene:

```
intentar {
    intentar {
        sea x = 1 / 0   # capturado por el intentar interior
    } rescatar e_interno {
        mostrar "interior: " + e_interno
    }
    # ejecución continúa aquí
} rescatar e_externo {
    mostrar "exterior: " + e_externo   # no se ejecuta si el interior captura
}
```

### 3.7 `intentar` sin `rescatar` — no válido

La forma `intentar { ... }` sin `rescatar` es un error sintáctico. `intentar` siempre
requiere al menos un `rescatar`.

---

## 4. Reglas semánticas

### 4.1 Ámbito del bloque `intentar`

Las variables declaradas dentro del bloque `intentar` **no** son visibles en `rescatar`
ni después de él. Cada bloque tiene su propio ámbito (ADR-0007 §1.6).

```
intentar {
    var x: entero = 5
} rescatar e {
    mostrar x   # error: x no está definida en este ámbito
}
```

### 4.2 Ámbito de la variable de error

La variable de error (`e` en `rescatar e`) es local al bloque `rescatar`. No es
visible fuera de él.

### 4.3 Retorno dentro de `intentar`

`retornar` dentro de `intentar` retorna de la función contenedora. Si hay un
`rescatar` pendiente, este **no** se ejecuta cuando el retorno es exitoso.

```
funcion intentar_division(a: entero, b: entero) -> texto {
    intentar {
        retornar texto(a / b)   # retorna directamente si no hay error
    } rescatar e {
        retornar "error: " + e
    }
}
```

### 4.4 Nuevas causas de error (`CausaError`)

No se añaden nuevas causas. `intentar`/`rescatar` es un mecanismo de captura de
errores existentes, no una fuente nueva.

---

## 5. Gramática ampliada

```ebnf
sentencia = ...
          | intentar_rescatar ;

intentar_rescatar = "intentar" , bloque ,
                    "rescatar" , [ IDENT ] , bloque ;
```

Nuevos tokens léxicos:

```
"intentar"  →  TipoToken::Intentar
"rescatar"  →  TipoToken::Rescatar
```

Ninguno de estos identificadores aparece en el código actual [INFERRED; requiere
verificación con grep antes de implementar].

---

## 6. Compatibilidad y migración

- **Sin rotura de código existente**: `intentar` y `rescatar` son identificadores
  válidos hoy; al volverse palabras clave, cualquier variable o función con ese nombre
  quedaría rota. [INFERRED: improbable; requiere grep de confirmación].
- **El comportamiento de errores no capturados no cambia**: si no hay `intentar`/`rescatar`,
  los errores siguen abortando el programa con el mensaje actual.

---

## 7. Criterios de aceptación

### Captura básica
- División por cero dentro de `intentar` → ejecuta `rescatar` con mensaje `"división entre cero"`.
- Código sin error en `intentar` → `rescatar` no se ejecuta.
- Error en `rescatar` → sube al `intentar` exterior o aborta.

### Alcance
- Variable declarada en `intentar` no visible en `rescatar` → error `VariableNoDefinida`.
- Variable declarada antes de `intentar` visible en `rescatar` → ok.

### Retorno
- `retornar` dentro de `intentar` retorna de la función → `rescatar` no se ejecuta.
- `retornar` dentro de `rescatar` retorna de la función → ok.

### Anidamiento
- `intentar` dentro de `intentar`: el error lo captura el más cercano.
- Error en `rescatar` interior sube al `intentar` exterior.

### `romper`/`continuar`
- `romper` dentro de `intentar` que está dentro de `para cada` → rompe el ciclo,
  no activa `rescatar`.

### Sintáctico
- `intentar { ... }` sin `rescatar` → error sintáctico.
- `rescatar` sin `intentar` → error sintáctico.
- `intentar` sin `{` en la misma línea → error sintáctico (ADR-0007 §1.6).

### Regresión
- Las 268 pruebas existentes siguen en verde.

---

## 8. Archivos a modificar (plan; sin cambios ahora)

| Archivo | Cambio |
|---------|--------|
| `prototipo/src/token.rs` | Añadir `TipoToken::Intentar`, `TipoToken::Rescatar` |
| `prototipo/src/lexer.rs` | Reconocer `intentar`, `rescatar` |
| `prototipo/src/parser.rs` | Sentencia `intentar`/`rescatar`; validar `rescatar` obligatorio |
| `prototipo/src/evaluador.rs` | Captura de `ErrorEjecucion` en el bloque `intentar` |
| `prototipo/tests/conformidad_errores.rs` | Nueva suite |

---

## 9. Riesgos y condiciones de revisión

| Riesgo | Probabilidad | Mitigación |
|--------|-------------|------------|
| `intentar`/`rescatar` silencia errores que el programador debería ver | Media | El bloque `rescatar` tiene que ser explícito; el error no desaparece sin código del programador |
| La variable de error como `texto` impide discriminar por tipo de error | Media | Es suficiente para Puerta C; `resultado<T>` en incremento 9 resolverá la discriminación tipada |
| `retornar` dentro de `intentar` puede confundir al programador sobre si `rescatar` se ejecuta | Baja | Documentar con ejemplo en tutorial; prueba específica en la suite |

**Condiciones de revisión:**
- Al implementar `resultado<T>` (incremento 9), revisar si `intentar`/`rescatar` sigue
  siendo necesario o si se convierte en azúcar sintáctica sobre `resultado<T>`.
- Si la evidencia de usabilidad muestra que el texto del error no es suficiente para
  discriminar casos, considerar exponer la causa tipada como valor del lenguaje.

---

## 10. Lo que este ADR no decide

- `resultado<T>` — diferido al incremento 9.
- Propagación automática de errores (operador `?` o similar).
- Errores definibles por el usuario (tipos de error personalizados).
- Múltiples cláusulas `rescatar` para distintos tipos de error.
- `finalmente` (bloque que siempre se ejecuta).
- Gestión de recursos garantizada ante error (RAII, `defer`, `con ... como`).
