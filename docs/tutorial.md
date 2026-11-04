# Tutorial de BASILISCO (español)

BASILISCO es un lenguaje de programación de propósito general con identidad en español. Este tutorial recorre los conceptos básicos usando la sintaxis real del prototipo (v0.6.0+).

## 1. Primer programa

```bsk
mostrar "Hola, BASILISCO!"
```

Ejecutar:

```bash
cd prototipo
cargo run -- run archivo.bsk
# o con el binario ya compilado:
./target/debug/basilisco run archivo.bsk
```

El comando `mostrar` imprime un valor y añade un salto de línea. `mostrar_sin_salto` imprime sin salto.

## 2. Variables y tipos

```bsk
sea nombre = "Ana"
var edad: entero = 30
const PI = 3.14159

mostrar nombre
mostrar edad
```

- `sea` declara una variable inmutable.
- `var` declara una variable mutable (puede reasignarse con `=`).
- `const` declara una constante.
- Tipos disponibles: `entero`, `real`, `decimal`, `texto`, `logico`, `nulo`.

## 3. Operadores

Aritméticos: `+`, `-`, `*`, `/`, `%`. Comparación: `==`, `!=`, `<`, `>`, `<=`, `>=`. Lógicos: `y`, `o`, `no`. Concatenación de texto con `+`.

```bsk
sea a = 10
sea b = 3
mostrar a + b        // 13
mostrar a % b        // 1
mostrar a > b y b > 0 // verdadero
```

## 4. Control de flujo

### Si / sino

```bsk
si edad >= 18 {
  mostrar "mayor de edad"
} sino {
  mostrar "menor de edad"
}
```

### Mientras

```bsk
sea i = 0
sea suma = 0
mientras i < 5 {
  i = i + 1
  suma = suma + i
}
mostrar suma   // 15
```

### Para cada

```bsk
para cada x en [1, 2, 3, 4] {
  mostrar x
}
```

### Romper / Continuar

```bsk
sea i = 0
mientras i < 10 {
  i = i + 1
  si i == 3 { continuar }
  si i == 7 { romper }
  mostrar i
}
```

## 5. Funciones

```bsk
funcion suma(a: entero, b: entero) -> entero {
  retornar a + b
}

funcion factorial(n: entero) -> entero {
  si n < 2 { retornar 1 }
  sino { retornar n * factorial(n - 1) }
}

mostrar suma(2, 3)        // 5
mostrar factorial(6)      // 720
```

Funciones sin tipo de retorno anotado infieren el tipo en la primera llamada.

## 6. Clausuras (funciones anónimas)

```bsk
sea doble = funcion (x) { retornar x * 2 }
mostrar doble(5)   // 10
```

### Captura de variables

```bsk
funcion crear_sumador(x) {
  funcion sumar(yy) {
    retornar x + yy
  }
  retornar sumar
}

sea sumar10 = crear_sumador(10)
mostrar sumar10(5)   // 15
```

### Clausuras mutables (contadores)

```bsk
funcion crear_contador() {
  sea x = 0
  funcion inc() {
    x = x + 1
    retornar x
  }
  retornar inc
}

sea c = crear_contador()
mostrar c()   // 1
mostrar c()   // 2
```

## 7. Colecciones

```bsk
sea arr = [1, 2, 3]
mostrar longitud(arr)          // 3
mostrar arr[1]                 // 2

// listas (mutables)
sea lista = [1, 2]
sea lista2 = agregar(lista, 3)  // [1, 2, 3]
sea lista3 = eliminar(lista2, 0) // [2, 3]

// rangos
para cada x en 1..5 {
  mostrar x   // 1 2 3 4
}
```

Operaciones de orden superior:

```bsk
sea duplicados = map([1, 2, 3], funcion (x) { retornar x * 2 })
sea pares = filtrar([1, 2, 3, 4, 5], funcion (x) { retornar x % 2 == 0 })
sea suma = reducir([1, 2, 3, 4], funcion (acc, x) { retornar acc + x }, 0)
```

## 8. Texto

```bsk
sea s = "Hola Mundo"
mostrar longitud(s)           // 10
mostrar mayusculas(s)         // HOLA MUNDO
mostrar minusculas(s)         // hola mundo
mostrar contiene(s, "Mundo")  // verdadero
mostrar empieza_con(s, "Hola") // verdadero
mostrar termina_con(s, "Mundo") // verdadero
mostrar subcadena(s, 0, 4)    // Hola
mostrar dividir(s, " ")       // ["Hola", "Mundo"]
mostrar unir(["a", "b"], "-") // a-b
mostrar reemplazar(s, "Mundo", "BASILISCO")
mostrar recortar("  hola  ")  // "hola"
mostrar repetir("ab", 3)      // "ababab"
mostrar invertir(s)           // "odnuM aloH"
```

Interpolación:

```bsk
sea nombre = "Ana"
mostrar "Hola, {nombre}!"
```

## 9. Módulos

### Usar (importar nombres específicos)

```bsk
// lib.bsk
publico funcion doble(x) { retornar x * 2 }

// main.bsk
usar "lib.bsk" { doble }
mostrar doble(21)   // 42
```

### Usar con alias

```bsk
usar "lib.bsk" como lib
mostrar lib.doble(7)   // 14
```

### Importar

```bsk
importar "lib.bsk" como l
mostrar l.doble(21)   // 42
```

## 10. Manejo de errores

```bsk
funcion dividir_seguro(a, b) {
  intentar {
    retornar a / b
  } rescatar e {
    mostrar "Error: {e}"
    retornar 0
  }
}

mostrar dividir_seguro(10, 0)   // Error: ... ; 0
```

## 11. Concurrencia (tareas)

```bsk
funcion tarea() -> entero {
  sea s = 0
  sea i = 1
  mientras i <= 100 {
    s = s + i
    i = i + 1
  }
  retornar s
}

sea id = iniciar_tarea("tarea")
sea local = 5
mostrar esperar_tarea(id) + local   // 5050 + 5 = 5055
```

## 12. JSON

```bsk
sea obj = json_parsear("{\"nombre\": \"Ana\", \"edad\": 30}")
mostrar json_obtener(obj, "nombre")   // Ana
mostrar json_serializar(obj)
```

## 13. Archivos

```bsk
sea f = abrir("datos.txt", "escribir")
escribir_linea(f, "hola")
cerrar(f)

sea g = abrir("datos.txt", "leer")
mostrar leer_linea(g)   // hola
cerrar(g)
```

Modos de `abrir`: `"leer"`, `"escribir"`, `"agregar"`.

## 14. FFI (interoperabilidad con C)

```bsk
sea id = cargar_ffi("./libejemplo.so")
mostrar llamar_ffi(id, "sumar", [3, 4], "entero")   // 7
descargar_ffi(id)
```

## 15. HTTP y fechas

```bsk
mostrar http_get("https://example.com")
mostrar ahora()           // milisegundos desde epoch
mostrar fecha_actual()    // YYYY-MM-DD
mostrar formatear_fecha(0, "%Y-%m-%d")  // 1970-01-01
```

## 16. Motor de ejecución

BASILISCO soporta dos motores:
- **Árbol (Tree-Walk)**: intérprete de referencia, siempre disponible.
- **VM**: máquina virtual basada en MIR, más rápida.

```bash
basilisco --motor=vm run archivo.bsk
# o por variable de entorno:
BASILISCO_MOTOR=vm basilisco run archivo.bsk
```

Por defecto se usa la VM. Si falla, se cae automáticamente al Tree-Walk.

## 17. Formateador

```bash
basilisco formatear archivo.bsk
```

## 18. Análisis

```bash
basilisco analizar archivo.bsk
```

## Más información

- Referencia de stdlib: `docs/stdlib.md`
- Guía de migración Tree-Walk → VM: `docs/migracion-vm.md`
- Especificación: `10-especificacion/`
- Glosario: `09-glosario/glosario.md`
