# Decisiones que no deben inventarse

Este documento protege el proyecto contra decisiones accidentales presentadas como si estuvieran aprobadas.

Antes de fijar cualquiera de estos puntos se requiere evidencia o ADR:
- lenguaje estático, dinámico o mixto;
- mutabilidad predeterminada de `sea`;
- constantes y su sintaxis;
- modelo de cadenas, normalización Unicode e índices;
- colecciones y semántica de copia/alias;
- genéricos, traits/interfaces y restricciones;
- excepciones, `Resultado` u otro modelo de errores;
- funciones, closures, recursión y orden de evaluación;
- gestión de memoria, GC, ownership o gestión explícita;
- AST interpretado, bytecode, compilación nativa o híbrida;
- concurrencia, async/await, canales y cancelación;
- FFI, ABI y reglas de ownership en fronteras nativas;
- módulos, paquetes, resolución y ejecución de scripts;
- sandbox, permisos, acceso a red y archivos;
- soporte oficial por plataforma;
- formato de paquetes, registry, firma y política de publicación;
- estabilidad de biblioteca estándar y API;
- compatibilidad semántica y migraciones.

Para cada decisión, registrar opciones, criterios, consecuencias, prueba que puede refutar la elección y condición para reconsiderarla. La aparición de una tecnología en un documento no la aprueba.
