# Delegación a agentes especialistas

Proporcionar al subagente: ID, alcance autorizado, fase, commit base, issues, criterios de aceptación, pruebas, módulos permitidos y acciones prohibidas.

El subagente devuelve: hallazgos, archivos inspeccionados/modificados, evidencia, pruebas reales, riesgos y estado propuesto.

El agente principal revisa el diff y consolida resultados. Ningún subagente puede aprobar fases ni autorizar trabajo posterior.
