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

Los bloques introducen ámbitos conforme al modelo léxico y no producen valor hacia el nivel superior; el resultado de `ejecutar_programa` es la última sentencia de expresión del nivel superior, si existe. `romper`, `continuar`, `para cada` y los rangos se definieron después en ADR-0012, pero no pertenecen a la gramática normativa de Core 0.3. El límite de 100 000 iteraciones de `mientras` es una protección provisional del prototipo conforme a ADR-0012; no es un requisito normativo de Core 0.3.

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

## 10. Límites explícitos y relación con el prototipo

Core 0.3 sigue siendo una propuesta normativa inicial y acotada. No declara conformidad del prototipo. La siguiente matriz registra la diferencia observable en el prototipo local verificado el 2026-10-25; es informativa y no amplía silenciosamente la norma:

| Área | Core 0.3 | Prototipo local | Estado para v0.4 |
|---|---|---|---|
| Números, variables, ámbitos, lógica y `si`/`sino`/`mientras` | Cubiertos por ADR-0005/0006/0007; detalle normativo pendiente de cierre | Implementados y con pruebas de conformidad | Incluidos |
| Funciones, parámetros y retorno | No definidos exhaustivamente en esta edición | Implementados; ADR-0008/0011 y suite propia | Incluidos, con límites documentados |
| Texto | No definido exhaustivamente como tipo y operaciones | Literal, concatenación, comparación y mostrar; suite propia | Incluido en operaciones básicas |
| Arreglos/listas | Fuera de la norma Core 0.3 | Tipos anotados, literales, indexación y asignación con pruebas | Incluidos solo en su forma básica |
| `para cada`, iteración de texto, rangos, `romper` y `continuar` | Fuera de la norma Core 0.3; ADR-0012 posterior | Implementación presente; casos límite de conformidad incompletos | Extensiones prototipales, fuera de la garantía v0.4 |
| E/S de texto y archivos | Fuera de la norma Core 0.3; ADR-0013 posterior | Implementación parcial | Extensión prototipal, fuera de la garantía v0.4 |
| `intentar`/`rescatar` | Fuera de la norma Core 0.3; ADR-0014 posterior | Implementación presente; suite de aceptación específica incompleta | Extensión prototipal, fuera de la garantía v0.4 |
| Módulos, paquetes, concurrencia, ABI/FFI y biblioteca estándar | No definidos | No disponibles como capacidades completas | Fuera de alcance |

Para incorporar una extensión a una versión garantizada se requiere especificación normativa, criterios positivos y negativos, pruebas de límites, revisión de seguridad cuando corresponda y actualización del registro de brechas. La existencia de un ADR aprobado y código ejecutable no basta para declarar conformidad.
