+++
title = "Seguridad"
description = "Modelo de seguridad, garantias y limitaciones de SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Seguridad

SSH-Frontière es un **componente de seguridad**. Su razon de ser es restringir lo que las conexiones SSH entrantes pueden hacer. Esta pagina documenta el modelo de seguridad, lo que se ha implementado y lo que no esta garantizado.

## Modelo de seguridad

### Principio fundamental: deny by default

Nada se ejecuta sin estar explicitamente configurado. Si un comando no esta en la whitelist TOML, se rechaza. No hay modo permisivo, no hay fallback hacia un shell.

### Tres capas de defensa en profundidad

| Capa | Mecanismo | Proteccion |
|------|-----------|------------|
| 1 | `command=` + `restrict` en `authorized_keys` | Fuerza el nivel de acceso, bloquea forwarding/PTY |
| 2 | SSH-Frontière (shell de inicio) | Valida el comando contra la whitelist TOML |
| 3 | Whitelist `sudo` en sudoers | Restringe los comandos de sistema privilegiados |

Incluso si un atacante compromete una clave SSH (capa 1), solo puede ejecutar los comandos autorizados en la whitelist TOML (capa 2). Incluso si elude la capa 2, solo puede elevar privilegios para los comandos autorizados en sudoers (capa 3).

### Analizador gramatical, no lista negra

SSH-Frontière **no es un shell**. La seguridad no se basa en un filtrado de caracteres (sin lista negra de `|`, `;`, `&`), sino en un **analizador gramatical**.

La gramatica esperada es `dominio accion [key=value ...]`. Todo lo que no respete esta estructura se rechaza. Los caracteres especiales entre comillas son contenido de argumento, no sintaxis — son validos.

`std::process::Command` ejecuta directamente, sin pasar por un shell intermedio. La inyeccion de comandos es **estructuralmente imposible**.

### Determinismo frente a agentes IA

Este funcionamiento es **determinista**: un comando dado produce siempre el mismo resultado de validacion, independientemente del contexto. Es una propiedad esencial cuando se trabaja con agentes de IA, cuya naturaleza es precisamente el **indeterminismo** — un modelo puede estar sesgado, o la cadena de produccion del agente puede estar comprometida, apuntando a los shells para recuperar informacion adicional o exfiltrar secretos. Con SSH-Frontière, un agente comprometido no puede eludir la whitelist, no puede inyectar comandos en un shell y no puede acceder a recursos no configurados. Es **estructuralmente imposible**.

## Lo que se ha implementado

### Lenguaje Rust

SSH-Frontière esta escrito en Rust, lo que elimina las clases de vulnerabilidades mas comunes en los programas de sistema:
- Sin buffer overflow
- Sin use-after-free
- Sin null pointer dereference
- Sin `unsafe` en el codigo (prohibido por la configuracion de lints en `Cargo.toml`: `unsafe_code = "deny"`)

### 399 tests cargo + 72 escenarios E2E SSH

El proyecto esta cubierto por **399 tests cargo** y **72 escenarios E2E SSH** adicionales:

| Tipo | Cantidad | Descripcion |
|------|----------|-------------|
| Tests unitarios | ~340 | Cada modulo se prueba de forma independiente (10 archivos `*_tests.rs`) |
| Tests de integracion | 50 | Escenarios stdio completos (ejecucion del binario) |
| Tests de conformidad | 1 (6 escenarios) | Validacion del contrato de interfaz JSON (ADR 0003) |
| Tests proptest | 8 | Tests de propiedades (fuzzing guiado por restricciones) |
| **Total cargo** | **399** | |
| Escenarios E2E SSH | 72 | Docker Compose con un servidor SSH real |
| Harnesses cargo-fuzz | 9 | Fuzzing no guiado (mutaciones aleatorias) |

Los tests E2E SSH cubren el protocolo completo, la autenticacion, las sesiones, la seguridad, la robustez y el logging. Se ejecutan en un entorno Docker Compose con un servidor SSH real.

### Auditoria de dependencias

- `cargo deny` en CI: verifica las licencias y las vulnerabilidades conocidas (base RustSec)
- `cargo audit`: auditoria de seguridad de las dependencias
- `cargo clippy` en modo pedantic: 0 warnings autorizados
- Solo 3 dependencias directas: `serde`, `serde_json`, `toml` — todas ampliamente auditadas por la comunidad Rust

### Control de acceso RBAC

Tres niveles de confianza jerarquicos:

| Nivel | Uso | Ejemplos |
|-------|-----|----------|
| `read` | Solo consulta | healthcheck, status, list |
| `ops` | Operaciones habituales | backup, deploy, restart |
| `admin` | Todas las acciones | configuracion, datos sensibles |

Cada accion tiene un nivel requerido. Cada conexion SSH tiene un nivel efectivo (via `--level` en `authorized_keys` o via autenticacion por token).

### Tags de visibilidad

Como complemento del RBAC vertical, los **tags** permiten un filtrado horizontal: un token con el tag `forgejo` solo ve las acciones etiquetadas con `forgejo`, incluso si tiene el nivel `ops`.

### Autenticacion por token

Dos modos de autenticacion:

- **Modo simple** (`challenge_nonce = false`): challenge-response `SHA-256(secret)` — el cliente demuestra que conoce el secreto
- **Modo nonce** (`challenge_nonce = true`): challenge-response `SHA-256(XOR_encrypt(secret || nonce, secret))` con el nonce enviado por el servidor. El nonce se regenera despues de cada autenticacion exitosa, impidiendo la reutilizacion de un proof interceptado

### Protecciones adicionales

- **Timeout** por comando con kill del process group (SIGTERM y luego SIGKILL)
- **Lockout** tras N intentos de autenticacion fallidos (configurable, por defecto: 3)
- **Ban de IP** opcional mediante comando externo configurable
- **Enmascaramiento** de argumentos sensibles en los logs (SHA-256)
- **Limite de tamano** en las salidas capturadas (stdout, stderr)
- **Limpieza de entorno**: `env_clear()` en los procesos hijos, solo `PATH` y `SSH_FRONTIERE_SESSION` se inyectan

## Lo que no esta garantizado

Ningun software es perfecto. Estas son las limitaciones conocidas y documentadas:

### Contador XOR de 8 bits

La implementacion criptografica utiliza un contador XOR con un keystream limitado a 8192 bytes. Es suficiente para el uso actual (proofs SHA-256 de 64 caracteres), pero no esta disenado para cifrar grandes volumenes.

### Fuga de longitud en la comparacion

La comparacion en tiempo constante puede revelar la longitud de los valores comparados. En la practica, los proofs SHA-256 siempre tienen 64 caracteres, lo que hace que esta fuga sea insignificante.

### Rate limiting por conexion

El contador de intentos de autenticacion es local a cada conexion SSH. Un atacante puede abrir N conexiones y disponer de N x `max_auth_failures` intentos. Recomendacion: combinar con fail2ban, `sshd MaxAuthTries` o reglas iptables.

### Reportar una vulnerabilidad

**No reporte vulnerabilidades a traves de los issues publicos.** Contacte directamente al mantenedor para una divulgacion responsable. El proceso se describe en la [guia de contribucion](@/contribuer.md).

## Dependencias

SSH-Frontière tiene una politica estricta de dependencias minimas. Cada crate externa se evalua segun una matriz ponderada (licencia, gobernanza, comunidad, tamano, dependencias transitivas).

| Crate | Version | Uso | Justificacion |
|-------|---------|-----|---------------|
| `serde` | 1.x | Serializacion/deserializacion | Estandar de facto en Rust, requerido para JSON y TOML |
| `serde_json` | 1.x | Respuestas JSON | Formato de salida del protocolo |
| `toml` | 0.8.x | Carga de la configuracion | Formato estandar en Rust para configuracion |

Dependencia de desarrollo: `proptest` (tests de propiedades unicamente, no se incluye en el binario final).

Fuentes autorizadas: **crates.io unicamente**. Ningun repositorio git externo autorizado. Politica verificada por `cargo deny`.
