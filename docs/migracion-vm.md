# Guía de migración: Tree-Walk → VM

## Resumen

BASILISCO tiene dos motores de ejecución:

1. **Tree-Walk (`arbol`)**: intérprete de referencia sobre el AST. Siempre disponible.
2. **VM (`vm`)**: máquina virtual basada en MIR (IR de medio nivel). Más rápida, con límites de pasos y profundidad.

Desde v0.6.0, el **motor por defecto es la VM**. El Tree-Walk sigue disponible como fallback y para diagnóstico.

## Selección del motor

```bash
# Por flag
basilisco --motor=vm run archivo.bsk
basilisco --motor=arbol run archivo.bsk

# Por variable de entorno
BASILISCO_MOTOR=vm basilisco run archivo.bsk
BASILISCO_MOTOR=arbol basilisco run archivo.bsk
```

Si la VM falla en un programa (error de ejecución, módulo no encontrado, etc.), el CLI cae automáticamente al Tree-Walk e imprime un aviso por stderr.

## Compatibilidad

Ambos motores soportan:
- Sintaxis completa (funciones, control de flujo, closures, colecciones).
- Biblioteca estándar (texto, colecciones, JSON, archivos, fechas, HTTP, FFI, tareas).
- Módulos (`usar { ... }`, `usar ... como alias`, `importar ... como alias`).
- Manejo de errores (`intentar`/`rescatar`).

## Diferencias conocidas

| Aspecto | Tree-Walk | VM |
|---|---|---|
| Errores de tipo en runtime | Detecta `TipoParametroIncompatible` | Detecta (desde v0.6.0) |
| Función sin retorno usada como valor | `E1010 RetornoAusente` | Detectado (E1010) |
| Límites de ejecución | Sin límite | 1.000.000 pasos, 1.000 niveles de profundidad |
| Rendimiento | Lento (interpretación AST) | Rápido (MIR + VM) |
| Backends nativo/WASM | No aplica | Prototipo (solo enteros/lógicos) |

## Cuándo usar cada motor

- **VM (default)**: producción, programas grandes, bucles intensivos.
- **Tree-Walk (`--motor=arbol`)**: depuración, diagnóstico de errores de tipo, desarrollo.

## Formateador y análisis

El formateador (`basilisco formatear`) y el analizador (`basilisco analizar`) son independientes del motor de ejecución.

## Verificación

Para comprobar que un programa se comporta igual en ambos motores:

```bash
BASILISCO_MOTOR=arbol basilisco run programa.bsk > out_arbol.txt
BASILISCO_MOTOR=vm basilisco run programa.bsk > out_vm.txt
diff out_arbol.txt out_vm.txt
```
