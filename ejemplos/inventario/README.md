# BASILISCO Inventario

Aplicación de consola escrita en BASILISCO. La lógica de negocio no depende de Python ni de Rust: utiliza funciones del lenguaje y su biblioteca estándar.

## Funciones

- Registro de productos con ID único, nombre, precio y existencia inicial.
- Listado y consulta por ID.
- Entradas de mercancía.
- Salidas con validación para impedir stock negativo.
- Cálculo del valor total del inventario.
- Persistencia automática en JSON.

## Ejecución

Desde el directorio `prototipo/`:

```bash
cargo run -- run ../ejemplos/inventario/main.bsk
```

Para forzar la VM:

```bash
cargo run -- --motor=vm run ../ejemplos/inventario/main.bsk
```

Si tu CLI acepta el selector después del subcomando, usa la primera forma equivalente que muestre `basilisco --help`.

## Modelo de datos

Cada producto se almacena como una lista:

`[id, nombre, precio, existencia]`

El archivo `datos/inventario.json` contiene el arreglo completo. Se eligió esta representación para mantener el modelo compatible con las operaciones de indexación y asignación del lenguaje, sin depender de mutación de campos de objetos.

## Estado de validación

El código está preparado contra las APIs documentadas de BASILISCO, pero debe ejecutarse en el checkout local para confirmar compatibilidad real de parser, VM y persistencia. No se declara todavía como suite aprobada.
