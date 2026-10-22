# Compatibilidad y versionado de Nexo

## Objetivo
Evitar que programas existentes cambien de significado sin aviso.

## Versiones separadas
Se versionan de forma diferenciada:
- especificación del lenguaje;
- implementación/runtime;
- CLI y herramientas;
- biblioteca estándar;
- formato de paquetes/manifiestos;
- APIs públicas y ABI cuando exista.

## Política propuesta
Usar versionado semántico para artefactos que tengan una API pública estable:
- cambio mayor: ruptura incompatible documentada;
- cambio menor: funcionalidad compatible;
- parche: corrección compatible.

Antes de declarar estabilidad, debe definirse qué artefactos siguen esta política y qué significa “compatible” para cada uno.

## Cambios incompatibles
Requieren:
1. motivo y alternativas;
2. ejemplo anterior/nuevo;
3. alcance de programas afectados;
4. estrategia de migración;
5. periodo de aviso cuando sea viable;
6. pruebas de compatibilidad y regresión;
7. notas de publicación.

## Cambios de semántica
Un cambio que altera resultado, tipo inferido, orden de evaluación, error observable o comportamiento de biblioteca es potencialmente incompatible aunque no cambie la gramática.

## Compatibilidad de paquetes
Un paquete debe declarar versiones compatibles del lenguaje y de sus dependencias. Los builds deben poder reconstruirse desde un lockfile con integridad verificada. El formato del lockfile debe versionarse.

## Estado
Política propuesta; requiere ADR y pruebas de herramientas antes de ser compromiso público.
