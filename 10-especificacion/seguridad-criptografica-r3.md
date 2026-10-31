# BASILISCO — Seguridad criptográfica R3

## 1. Objetivo

Definir el sistema de confianza criptográfica del ecosistema de paquetes BASILISCO, compatible con R1 y R2.

## 2. Algoritmos

- Firmas: Ed25519.
- Integridad: SHA-256.
- Claves públicas de confianza por registro.

## 3. Serialización canónica

Se adopta JSON Canonicalization Scheme (RFC 8785) para serializar los metadatos firmados.

Los bytes firmados son el hash SHA-256 de la serialización canónica en UTF-8.

La firma Ed25519 se codifica en base64 estándar.

### Bytes exactos de firma

1. Serializar los metadatos mediante JCS RFC 8785.
2. Codificar el resultado como UTF-8.
3. Calcular SHA-256 del texto UTF-8, obteniendo 32 bytes binarios.
4. Firmar con Ed25519 los 32 bytes binarios del digest.
5. Representar la firma resultante (64 bytes) en Base64 estándar.

## 4. Campos firmados

Metadatos firmados incluyen:
- namespace
- name
- version
- sha256 (del artefacto BPK)
- published_at
- registry_id
- key_id
- nonce (identificador único de publicación)
- expires_at (vigencia)

## 5. Identidad de registros y confianza inicial

Cada registro tiene:
- Identificador único.
- URL canónica.
- Clave pública inicial distribuida fuera de banda.

El cliente BASILISCO mantiene un almacén de claves públicas confiables por registro.

## 6. Generación y protección de claves

- Claves privadas generadas localmente.
- Almacenamiento protegido con permisos restrictivos.
- Respaldo responsabilidad del propietario.
- Clave privada nunca transmitida.

## 7. Rotación y revocación

### Rotación autenticada

- La nueva clave debe estar firmada por la clave anterior, o por una autoridad de confianza del registro.
- Se define un periodo de transición durante el cual ambas claves son válidas.
- Si la clave anterior está comprometida, la rotación debe realizarse mediante un mecanismo de recuperación administrativa del registro, no mediante la clave comprometida.

### Recuperación de claves

- Se establece una autoridad de recuperación independiente y confiable.
- El cliente valida una nueva clave cuando la anterior está comprometida mediante la firma de la autoridad de recuperación.
- No se acepta como prueba suficiente una declaración firmada únicamente por el registro comprometido.

### Revocación

- Revocación administrativa: el registro marca la clave como no confiable.
- Revocación por compromiso: la clave se revoca inmediatamente.

### Consecuencias de revocación

- Instalaciones existentes: se conservan pero se marcan como comprometidas.
- Nuevas instalaciones: se rechazan si usan la clave revocada.
- Actualizaciones: requieren reinstalación con una clave válida.
- Se mantiene evidencia de la clave y firma con que se instaló cada paquete.

## 8. Verificación

- Calcular SHA-256 de los metadatos canónicos.
- Verificar firma Ed25519 con la clave pública del registro.
- Verificar que la clave no esté revocada.
- Verificar correspondencia con registro y artefacto.

## 9. Política ante claves desconocidas, revocadas e inválidas

- Clave desconocida: `autenticidad_pendiente`.
- Clave revocada: `verificacion_fallida`.
- Firma inválida: `verificacion_fallida`.
- Integridad fallida: `verificacion_fallida`.

Nunca asumir autenticidad sin verificación exitosa.

## 10. Protección contra replay y sustitución

- Cada firma incluye un nonce único y un timestamp.
- El cliente rechaza firmas con nonce repetido.
- Los metadatos incluyen versión exacta y hash del artefacto.
- La sustitución requiere nueva firma válida.
- Se establece una política de vigencia temporal: las firmas expiradas se rechazan.
- Se evita aceptar una publicación antigua como si fuera nueva mediante el nonce y la versión.

### Persistencia anti-replay

- El cliente registra los nonces aceptados por registro.
- Los nonces se persisten para evitar su reutilización tras reinicios.
- Si se pierde el historial de nonces, el cliente debe tratar como no verificadas las firmas cuyo nonce no pueda confirmarse, y requerir revalidación.
- Se establecen controles contra downgrade de versiones: no se acepta una versión menor que la ya instalada, salvo instalación explícita solicitada por el usuario.

## 11. Integración con R1

Los estados de verificación del lock file R1 se mapean así:

- `integridad_verificada`: SHA-256 coincide.
- `autenticidad_verificada`: firma válida con clave confiable.
- `autenticidad_pendiente`: sin firma o sin clave confiable.
- `verificacion_fallida`: integridad o autenticidad falló.

La causa específica de cada fallo se conserva en el campo `verification` del lock file.

## 12. Modelo de amenazas

- Atacante puede sustituir paquetes.
- Atacante puede reutilizar firmas antiguas.
- Atacante puede comprometer claves.
- Atacante puede crear registros falsos.

Mitigaciones: firmas, claves de confianza, nonces, revocación.

## 13. Vectores de prueba conceptuales

1. Firma válida: metadatos sin modificar → aceptado.
2. Metadatos alterados: firma inválida → rechazado.
3. Clave revocada: metadatos firmados por clave revocada → rechazado.
4. Firma repetida: mismo nonce → rechazado.
5. Paquete sustituido: hash diferente → rechazado.
6. Rotación válida: nueva clave firmada por la anterior → aceptado.
7. Rotación no autorizada: nueva clave sin autorización → rechazado.
8. Clave comprometida: rotación mediante recuperación administrativa → aceptado.
9. Nonce repetido: firma con nonce ya usado → rechazado.
10. Publicación vencida: firma expirada → rechazado.
11. Paquete instalado posteriormente afectado por revocación: marcado como comprometido.
12. Pérdida del historial anti-replay: firmas con nonce desconocido se tratan como no verificadas.
13. Downgrade: intento de instalar una versión menor sin autorización explícita → rechazado.
14. Recuperación sin autorización: nueva clave firmada solo por el registro comprometido → rechazado.
15. Revocación administrativa vs compromiso: la administrativa permite transición; la de compromiso revoca inmediatamente.

## 14. Criterios de aceptación

- Firmas Ed25519 definidas.
- Claves públicas por registro.
- Verificación antes de instalar.
- Estados de verificación documentados.
- Compatibilidad con R1/R2.

## 15. Restricciones

- Solo documentación.
- No modificar código.
- No implementar.
- No introducir dependencias.

## 16. Informe de decisiones y pendientes

### Decisiones
- RFC 8785 para serialización canónica.
- Ed25519 y SHA-256.
- Firmas sobre hash de metadatos.
- Claves por registro.
- Estados de verificación explícitos.
- Nonces y timestamps contra replay.
- Rotación autenticada y revocación administrativa.
- Protección contra replay mediante nonce y vigencia temporal.

### Pendientes para implementación
- Generación y almacenamiento de claves.
- Verificación en cliente.
- Rotación y revocación.
- Protección contra repetición.

---

**Fin de la especificación R3.**

**BASILISCO — Cryptographic Security Specification**