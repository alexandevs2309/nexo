# BASILISCO 0.5.0 (en desarrollo)

Intérprete experimental de un lenguaje de programación con sintaxis orientada al español. Esta versión permite ejecutar programas pequeños desde la línea de comandos; el lenguaje sigue en desarrollo.

## Instalación

Cuando BASILISCO 0.5.0 esté publicado en crates.io:

```sh
cargo install basilisco --version 0.5.0
```

También puedes compilarlo desde el repositorio:

```sh
git clone https://github.com/alexandevs2309/nexo.git
cd nexo/prototipo
cargo install --path .
```

## Ejecutar un programa

En una carpeta vacía, crea un archivo inicial con:

```sh
basilisco nuevo
```

Esto crea `hola.bsk`. También puedes indicar el nombre: `basilisco nuevo saludo` crea `saludo.bsk`. El comando no sobrescribe archivos existentes.

Ejecuta el programa con:

```sh
basilisco run hola.bsk
```

El comando `bsk` es un alias. `basilisco --help` muestra la ayuda y `basilisco --version` muestra la versión instalada.

## Ejemplo

```basilisco
funcion doble(n) {
  retornar n * 2
}

sea total = doble(21)
total
```

Guarda el programa en un archivo `.bsk` y ejecútalo con `basilisco run archivo.bsk`. La salida es `42`.

Consulta el [tutorial](tutorial/01-primer-programa.md) y los [ejemplos](examples/) incluidos. El código se distribuye bajo [Apache-2.0](LICENSE). La especificación completa del lenguaje y sus límites están documentados en el [repositorio](https://github.com/alexandevs2309/nexo).
