# ADR-0016 — Licencia de publicación de BASILISCO

- Estado: **Aprobado para la candidata v0.4**, sujeto a auditoría de procedencia del código y dependencias.
- Fecha: 2026-10-25
- Responsable: responsable del proyecto BASILISCO
- Relacionado: PR #1; `LICENSE`; `prototipo/Cargo.toml`.

## Contexto

BASILISCO busca ser un lenguaje de programación abierto y gratuito. Antes de distribuir una versión pública se necesita una licencia explícita y metadatos coherentes en el paquete Cargo. El PR #1 prepara la versión candidata 0.4.0 y la matriz multiplataforma, y aún no publica ni etiqueta esa versión.

## Opciones consideradas

### O1 — MIT

Licencia permisiva breve, adecuada para maximizar la sencillez de reutilización. No contiene una concesión expresa de patentes tan desarrollada como Apache-2.0.

### O2 — Apache License 2.0

Licencia permisiva para uso, modificación, redistribución y uso comercial, sujeta a sus condiciones. Incluye concesiones expresas de derechos de patente y reglas sobre avisos y redistribución.

### O3 — GPL-3.0

Licencia de copyleft que puede exigir ofrecer el código fuente bajo los términos de la GPL al distribuir determinadas obras derivadas. No es necesaria para el objetivo actual de máxima flexibilidad de adopción.

## Decisión

Se elige **Apache License 2.0** para el código de BASILISCO. El repositorio incluye el archivo `LICENSE`, y el paquete `basilisco` declara `license = "Apache-2.0"` en `prototipo/Cargo.toml`.

La licencia permite usos comerciales y distribución de derivados sujetos a las condiciones de Apache-2.0. La licencia del proyecto no otorga derechos sobre la marca BASILISCO.

## Condiciones antes de la publicación

1. Confirmar que el titular del proyecto tiene derecho a licenciar todo el código y documentación que se distribuyen, incluidas contribuciones previas.
2. Revisar dependencias, avisos y materiales de terceros para cumplir sus respectivas licencias.
3. Volver a ejecutar y verificar CI después de incorporar estos cambios de licencia y metadatos.
4. Verificar que `cargo package` y la instalación funcionan sobre el commit final.
5. No etiquetar ni publicar la versión hasta que esas comprobaciones estén completas.

## Consecuencias

- El paquete declara una licencia explícita que Cargo puede interpretar.
- Se permite la adopción abierta y comercial con las condiciones de Apache-2.0.
- Las contribuciones futuras deberán aceptarse con claridad sobre los términos de licencia; esta decisión no sustituye una política de contribuciones.
- La aprobación de la licencia no significa que la versión esté publicada ni que todas las obligaciones de procedencia y dependencias ya estén auditadas.

## Revisión

Reabrir este ADR si se identifica código de procedencia incierta, una licencia de terceros incompatible, un requisito de copyleft o una decisión estratégica que requiera otra política de distribución.
