# Modelos de ejecución — no se ha elegido uno
## A. Intérprete sobre AST
Prototipo potencialmente sencillo; puede limitar optimización y rendimiento.

## B. Bytecode y máquina virtual
Permite separar frontend y ejecución; exige diseñar VM, formato, depuración y compatibilidad.

## C. Compilación nativa
Puede aprovechar optimizaciones de CPU; aumenta complejidad de toolchains, ABI, compilación cruzada y depuración.

## D. Híbrido
Combina modos según objetivos; aumenta la matriz de componentes y pruebas.

## Criterios
Simplicidad inicial, rendimiento medido, portabilidad, depuración, tamaño de runtime, seguridad, coste de mantenimiento e interoperabilidad. Decidir con ADR y prototipos comparables.
