# BASILISCO Core 0.3 — Especificación inicial del núcleo

> **Nota de identidad:** el nombre histórico de esta edición es «Nexo Core 0.3». La identidad objetivo es **BASILISCO** (ADR-0009); se conserva el nombre histórico en referencias y archivos para no romper enlaces.

**Estado:** propuesta normativa para revisión; no declarar aprobada sin ADR/revisión formal.  
**Ámbito:** expresiones, valores básicos, variables, bloques y control de flujo descritos por el prototipo de referencia.

## 1. Convenciones
Las palabras “debe”, “no debe”, “debería” y “puede” expresan, respectivamente, obligación, prohibición, recomendación y permiso. Una implementación conforme debe documentar las extensiones que acepte fuera de esta especificación.

## 2. Tipos y valores de referencia
El prototipo documentado reconoce:
- `entero`: entero con signo de 64 bits.
- `decimal`: punto fijo de escala 9, con representación descrita como `DECIMAL(28,9)`.
- `real`: coma flotante binaria de 64 bits.
- `lógico`: `verdadero` o `falso`.

La representación interna no debe confundirse con la sintaxis superficial. Los límites exactos del tipo decimal, los errores de overflow y la conversión desde literales deben quedar expresados en una tabla normativa y comprobados por tests antes de cerrar esta sección.

## 3. Literales
La sintaxis de literales debe ser coherente con la gramática y el lexer real. Como mínimo, la norma final debe precisar:
- forma de enteros y decimales;
- separador decimal;
- exponentes y sufijos, si existen;
- separadores visuales permitidos;
- límites y diagnóstico de literales fuera de rango;
- tratamiento de signos: el signo puede ser operador unario, no necesariamente parte del token literal;
- escapes, comillas y Unicode para cadenas cuando se incorporen.

No aceptar una forma de literal en la documentación si el lexer no la reconoce.

## 4. Variables y ámbito
La forma documentada de declaración usa `sea`, por ejemplo `sea total = 0`. La especificación completa debe decidir si `sea` declara siempre una variable mutable, si existen constantes y cómo se expresa la inmutabilidad. Hasta que se cierre esa decisión, no inferir garantías de inmutabilidad por el nombre de la palabra clave.

Las asignaciones deben resolver el identificador en el ámbito léxico correspondiente. Un identificador no declarado debe producir un error de análisis semántico, no un valor silencioso. El sombreado de nombres, la inicialización antes del uso y las declaraciones duplicadas en el mismo ámbito requieren reglas explícitas y casos de prueba.

## 5. Operadores numéricos
Los operadores aritméticos y su precedencia deben ser fijados en la gramática normativa. Según las decisiones documentadas del prototipo:
- la división de enteros trunca hacia cero;
- el resto conserva el signo del dividendo;
- la división entre cero produce error;
- `i64::MIN % -1` produce `0` para evitar overflow en ese caso especial;
- el redondeo decimal usa half-even donde aplique;
- las conversiones implícitas solo se permiten cuando se consideran sin pérdida según las reglas de conversión aprobadas.

La tabla final debe enumerar operandos admitidos, tipo del resultado, overflow, división por cero, precisión y conversiones para cada operador.

## 6. Comparaciones y lógica
Los operadores de comparación documentados son `==`, `!=`, `<`, `<=`, `>` y `>=`. Las comparaciones entre `decimal` y `real` no están permitidas en el diseño de referencia. La igualdad y orden entre tipos deben definirse sin conversiones silenciosas que cambien el resultado.

Los operadores `y` y `o` deben evaluar de izquierda a derecha con cortocircuito; `no` es negación lógica. Las condiciones de `si` y `mientras` deben ser de tipo `lógico`; no se permite convertir automáticamente números, cadenas o valores ausentes en verdadero/falso.

## 7. Control de flujo
La sintaxis de referencia contempla:
- `si condición { ... }`
- `sino { ... }`
- `sino si condición { ... }`
- `mientras condición { ... }`

Los bloques introducen ámbitos conforme al modelo léxico y no producen valor hacia el nivel superior; el resultado de `ejecutar_programa` es la última sentencia de expresión del nivel superior, si existe. `romper` y `continuar` no forman parte del incremento 4 documentado; no deben prometerse hasta especificarlos y probarlos. El prototipo impone un límite de 100 000 iteraciones por ciclo `mientras` y emite un error tipado si se supera. La especificación final debe decidir si ese límite es normativo, configurable o solo protección del prototipo.

## 8. Errores
Los errores del lenguaje deben ser resultados controlados, no pánicos internos. Cada error visible debería incluir:
- categoría estable;
- ubicación de origen;
- mensaje en español;
- explicación de la regla incumplida;
- sugerencia solo cuando sea segura y no altere el significado silenciosamente.

Los identificadores internos de error deben ser estables aunque cambie el texto traducido.

## 9. Orden de evaluación
El orden de evaluación es parte de la semántica observable. Para los operadores lógicos, el cortocircuito es obligatorio. Para llamadas, asignaciones, operadores con efectos y acceso a colecciones, el orden debe fijarse antes de incorporarlos al núcleo.

## 10. Límites explícitos
El estado actual del prototipo (base `main` `a0c1a22` más cambios locales del 2026-10-22) es: `funcion`, `retornar`, `mostrar` y los valores de texto están implementados en parser/evaluador; ADR-0008 está **aprobado e implementado**. La gramática normativa de esta edición todavía no se actualizó de forma exhaustiva para esas construcciones. Esta versión no define aún de forma exhaustiva:
- funciones, recursión, retorno y cierres;
- colecciones y mutabilidad de sus elementos;
- cadenas como tipo normativo;
- errores recuperables y propagación;
- módulos, paquetes y resolución de dependencias;
- concurrencia, asincronía y memoria;
- ABI/FFI, backend nativo o bytecode;
- biblioteca estándar y garantías de plataforma.

Estos temas constan en el registro de brechas. No deben anunciarse como capacidades completas del lenguaje hasta cerrar especificación, implementación y pruebas.
