# Semántica, nombres y alcance
Debe definirse el significado de cada expresión y sentencia; orden de evaluación; resolución y sombreado de nombres; inicialización; referencias no definidas; verdad/falsedad; conversiones; retorno, ruptura y continuación; propagación de errores y efectos secundarios.

Cuando el orden de evaluación pueda cambiar el resultado, debe estar definido. El comportamiento no debe depender accidentalmente del sistema operativo, de una tabla hash o de una optimización.

Cada regla normativa debe incluir ejemplos, contraejemplos y pruebas.

## Captura de entorno en clausuras

BASILISCO captura el entorno por snapshot en el momento de definición de la
clausura. Las variables capturadas se almacenan en un entorno propio y
mutable para esa clausura. Las modificaciones persisten entre llamadas de la
misma clausura, pero no se comparten con el entorno externo ni con otras
clausuras creadas independientemente. La captura por referencia compartida
con el entorno externo queda como trabajo futuro.
