# Paquetes y dependencias (prototipo v0.5)

Estado: **implementación local mínima y verificada**. No hay descarga desde red,
registro central ni verificación criptográfica. Esta capacidad es una extensión
del prototipo y no forma parte de una garantía de versión.

## Manifiesto

Cada proyecto declara su identidad y sus dependencias en `basilisco.toml`:

```toml
[paquete]
nombre = "mi-proyecto"
version = "0.1.0"

[dependencias]
matematica = "0.1.0"
util = { version = "0.2.0", ruta = "../util" }
```

Reglas:

- `[paquete].nombre` es obligatorio y solo admite letras ASCII, dígitos, `-` y `_`
  (máximo 64 caracteres, debe empezar por letra o dígito).
- `[paquete].version` es opcional; si falta se usa `0.0.0`.
- En `[dependencias]`, el valor puede ser una versión entre comillas o una tabla
  en línea con `version` y `ruta` (alias `path`).
- Las versiones son **exactas**; `*` significa cualquiera. No hay rangos semver.
- Se rechazan claves o tablas desconocidas, dependencias duplicadas y valores sin
  comillas.

## Resolución local

Para cada dependencia se prueba, en orden:

1. La `ruta` explícita del manifiesto (relativa al manifiesto).
2. `<proyecto>/paquetes/<nombre>`.
3. `<proyecto>/vendor/<nombre>`.
4. `<proyecto>/<nombre>`.

Cada ruta puede ser un archivo `.bsk` o un directorio. En un directorio se busca,
en orden: `lib.bsk`, `src/lib.bsk`, `principal.bsk` y `<nombre>.bsk`.

Si el paquete encontrado tiene su propio `basilisco.toml`, se comprueban su
`nombre` y su `version` contra lo solicitado. Una discrepancia es un error de
resolución, no una advertencia.

## Bloqueo

`basilisco instalar` resuelve las dependencias y escribe `basilisco.lock` con
rutas relativas al proyecto:

```toml
# Generado por basilisco. No editar a mano.
version = 1

[[dependencia]]
nombre = "matematica"
version = "0.1.0"
ruta = "paquetes/matematica/lib.bsk"
```

`importar "matematica"` consulta primero el bloqueo del directorio de trabajo.
Si el bloqueo no existe, se mantienen las rutas convencionales de importación.

## Comandos

```bash
basilisco instalar [ruta-del-proyecto]   # resuelve y escribe basilisco.lock
basilisco paquetes [ruta-del-proyecto]   # lista el contenido del bloqueo
```

`instalar` falla con código 1 si falta el manifiesto, una dependencia no existe o
las versiones no coinciden. `paquetes` falla si no hay bloqueo.

## Ejemplo completo

```text
proyecto/
├── basilisco.toml
├── basilisco.lock
├── main.bsk
└── paquetes/
    └── mate/
        ├── basilisco.toml
        └── lib.bsk
```

```basilisco
# paquetes/mate/lib.bsk
exportar funcion duplicar(x: entero) -> entero {
  retornar x * 2
}
```

```basilisco
# main.bsk
importar "mate"
mostrar duplicar(21)   # 42
```

## Límites conocidos

- Las rutas del bloqueo y de `importar` se resuelven respecto al **directorio de
  trabajo actual**, no al archivo que importa. Ejecutar desde otra carpeta exige
  pasar por el directorio del proyecto.
- No hay descarga, actualización ni cache de paquetes remotos.
- No hay verificación de integridad ni firmas (GAP-020).
- No hay resolución transitiva: las dependencias de un paquete no se instalan
  automáticamente.
- No hay rangos de versión ni selección de múltiples versiones simultáneas.
- La caché de módulos del evaluador es por proceso: un módulo se ejecuta una vez
  aunque se importe varias veces.
