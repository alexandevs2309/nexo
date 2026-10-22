# Puertas de liberación

## Puerta A — Diseño revisable
- alcance del incremento declarado;
- gramática y semántica documentadas;
- alternativas y riesgos registrados;
- requisitos trazables;
- ejemplos sin contradicciones conocidas.

## Puerta B — Prototipo verificable
- build reproducible;
- pruebas positivas y negativas;
- errores controlados;
- CI identificada por commit;
- límites y exclusiones documentados.

## Puerta C — Núcleo utilizable
- variables, tipos, funciones y control de flujo especificados e implementados según alcance;
- suite de conformidad;
- diagnósticos con ubicación;
- tutorial ejecutable;
- tests de regresión y fuzzing básico.

## Puerta D — Distribución experimental
- instalación/desinstalación verificadas;
- matriz de plataforma;
- política de dependencias;
- threat model actualizado;
- errores conocidos y recuperación documentados.

## Puerta E — Estabilidad
- compatibilidad definida;
- política de versionado y migración;
- build y artefactos reproducibles;
- documentación de API;
- seguridad y respuesta a vulnerabilidades;
- usuarios piloto y tareas representativas completadas;
- soporte de plataforma medido y publicado.

Una puerta no se supera por declaración manual: debe adjuntarse evidencia reproducible. El estado de una puerta debe incluir commit, fecha, responsable, comandos, artefactos y excepciones.
