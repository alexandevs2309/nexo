# Visión de BASILISCO

**Versión del documento:** 1.0  
**Fecha:** 2026-11-05  
**Estado:** APROBADO

---

## 1. Qué es BASILISCO

BASILISCO es un lenguaje de programación de propósito general con sintaxis en español, diseñado para ser legible, seguro y evolucionar con integridad técnica.

No es solo un lenguaje. Es un ecosistema: compilador, intérprete, VM, gestor de dependencias, registro de paquetes y herramientas de desarrollo que trabajan juntos bajo convenciones oficiales.

---

## 2. Posición actual

El prototipo en `prototipo/` implementa el pipeline completo:

```
Lexer → Parser → Análisis semántico → HIR → MIR → VM
```

Con motor tree-walk como respaldo, optimizador MIR básico, CLI funcional, gestor de paquetes local y remoto, y registro de paquetes sobre PostgreSQL.

**Lo que existe hoy y funciona:**
- Ejecución de scripts `.bsk` con la VM como motor por defecto
- Funciones, recursión, closures, lambdas, módulos, excepciones
- Colecciones (arreglos, listas, rangos), tipos básicos
- `bsk nuevo proyecto <nombre>` genera estructura mínima
- `bsk run`, `bsk analizar`, `bsk formatear`, `bsk instalar`
- Registro remoto de paquetes con firma Ed25519 y SHA-256
- 782 tests automatizados, 0 fallos

**Lo que no existe todavía:**
- Genéricos, traits, OOP
- LSP, debugger, extensión IDE
- Sistema de plantillas seleccionables
- Ejemplos de aplicaciones completas y mantenidas
- REPL, sandbox, canales de concurrencia
- Backend nativo para programas reales (solo tipos entero/lógico hoy)

---

## 3. Hacia dónde va

BASILISCO debe proveer una experiencia de desarrollo comparable, en sus respectivos ámbitos, a la que ofrecen Cargo/Rust, Go, Angular CLI y Django.

No se pretende copiar sus arquitecturas. Se adoptan sus buenas prácticas de experiencia de desarrollo:

- **Cargo/Rust:** manifiesto declarativo, reproducibilidad de builds, integridad de dependencias.
- **Go:** un comando para todo (`go build`, `go test`, `go run`), convenciones que eliminan configuración innecesaria.
- **Angular CLI:** generación de proyectos con estructura profesional desde el primer día.
- **Django:** separación de responsabilidades clara (modelos, servicios, vistas), proyecto listo para producción desde la plantilla.

La implementación de estas capacidades es gradual. Cada etapa del ROADMAP es una dependencia técnica de la siguiente, no una lista de deseos.

---

## 4. Principios

1. **Funcionalidad antes que estética.** Las plantillas generan aplicaciones que corren. Los ejemplos son ejecutables y probados, no tutoriales decorativos.
2. **Honestidad sobre el estado.** La documentación distingue siempre entre IMPLEMENTADO, EXPERIMENTAL y FUTURO.
3. **Convenciones sobre configuración.** El ecosistema tiene una forma oficial de hacer cada cosa. Las alternativas se justifican explícitamente.
4. **Compatibilidad hacia adelante.** El sistema de proyectos evoluciona sin romper proyectos existentes.
5. **Verificabilidad.** Cada decisión de diseño está en un ADR. Cada capacidad tiene un test que la demuestra.
6. **Español nativo.** La sintaxis, la documentación y los mensajes de error son en español. No es una traducción: es el diseño original.

---

## 5. El ecosistema completo (visión a largo plazo)

```
bsk new mi-app                    # crear proyecto
bsk new mi-api --template rest-api   # desde plantilla
bsk run                           # ejecutar
bsk build                         # compilar a nativo/WASM
bsk test                          # pruebas
bsk add nombre-paquete            # dependencias
bsk publish                       # publicar al registro
bsk doc                           # generar documentación
```

Cada comando existe hoy en forma parcial o total. El estado actual de cada uno está en [ROADMAP.md](../ROADMAP.md) y en el [registro de brechas](../10-especificacion/registro-de-brechas.md).

---

## 6. Referencias

- Estado técnico detallado: [AUDITORIA_MAESTRA.md](../AUDITORIA_MAESTRA.md)
- Decisiones arquitectónicas: `00-gobernanza/adr/`
- Hoja de ruta por etapas: [ROADMAP.md](../ROADMAP.md)
- Registro de brechas: [10-especificacion/registro-de-brechas.md](../10-especificacion/registro-de-brechas.md)
- Sistema de proyectos: [PROJECT_SYSTEM.md](PROJECT_SYSTEM.md)
- Sistema de plantillas: [TEMPLATE_SYSTEM.md](TEMPLATE_SYSTEM.md)
- Ejemplos oficiales: [OFFICIAL_EXAMPLES.md](OFFICIAL_EXAMPLES.md)
- Convenciones: [PROJECT_CONVENTIONS.md](PROJECT_CONVENTIONS.md)
