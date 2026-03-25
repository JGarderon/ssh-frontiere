# SSH Frontière v3.0.0

**Website**: [pages.nothus.fr/ssh-frontiere](https://pages.nothus.fr/ssh-frontiere/es/)

[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.md)

Shell de inicio de sesión SSH restringida escrita en Rust — un único punto de entrada seguro para todas las conexiones SSH entrantes en un servidor.

SSH Frontière reemplaza la shell predeterminada (`/bin/bash`) en `/etc/passwd` y actúa como un **despachador seguro**: valida cada comando SSH contra una lista blanca TOML, aplica control de acceso RBAC de 3 niveles y devuelve resultados como JSON estructurado a través de un protocolo basado en cabeceras sobre stdin/stdout.

## Propósito

SSH Frontière es un **componente de seguridad** diseñado para cuentas de servicio SSH:

- **Runners CI/CD** (Forgejo Actions, GitHub Actions): operaciones de infraestructura desde contenedores
- **Agentes de IA** (Claude Code, etc.): acceso controlado al servidor con niveles de confianza
- **Mantenimiento automatizado**: copias de seguridad, despliegues, healthchecks

El programa es **síncrono y one-shot**: SSH crea un nuevo proceso para cada conexión, el despachador valida y ejecuta, luego termina. Sin daemon, sin async, sin Tokio.

## Instalación

### Requisitos previos

- Rust 1.70+ con el target `x86_64-unknown-linux-musl`
- `make` (opcional, para atajos)

### Compilación

```bash
# Via make
make release

# O directamente
cargo build --release --target x86_64-unknown-linux-musl
```

El binario estático resultante (`target/x86_64-unknown-linux-musl/release/ssh-frontiere`, ~1-2 MB) puede desplegarse sin dependencias del sistema.

```bash
sudo cp target/x86_64-unknown-linux-musl/release/ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere
```

## Configuración TOML

Archivo predeterminado: `/etc/ssh-frontiere/config.toml`.
Sobreescribir: `--config <ruta>` o variable de entorno `SSH_FRONTIERE_CONFIG`.

### Ejemplo completo

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300          # Timeout predeterminado (segundos)
default_level = "read"         # Nivel RBAC predeterminado
mask_sensitive = true           # Enmascarar argumentos sensibles en los logs
max_stdout_chars = 65536       # Límite de stdout capturado
max_stderr_chars = 16384       # Límite de stderr capturado
max_output_chars = 131072      # Límite global absoluto
timeout_session = 3600         # Timeout de keepalive de sesión (segundos)
max_auth_failures = 3          # Intentos de autenticación antes del bloqueo
log_comments = false           # Registrar comentarios del cliente
ban_command = ""               # Comando de bloqueo de IP (ej: "/usr/sbin/iptables -A INPUT -s {ip} -j DROP")

# --- Autenticación RBAC (opcional) ---

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Secreto codificado en Base64 con prefijo b64:
level = "ops"                                # Nivel otorgado por este token

[auth.tokens.admin-deploy]
secret = "b64:c2VjcmV0LWFkbWluLWRlcGxveQ=="
level = "admin"

# --- Dominios y acciones ---

[domains.forgejo]
description = "Infraestructura de la forja Git"

[domains.forgejo.actions.backup-config]
description = "Hacer copia de seguridad de la configuración de Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.deploy]
description = "Desplegar una versión"
level = "admin"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {version}"

[[domains.forgejo.actions.deploy.args]]
name = "version"
type = "enum"
values = ["latest", "stable", "canary"]

[domains.infra]
description = "Infraestructura del servidor"

[domains.infra.actions.healthcheck]
description = "Verificación de estado"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"
args = []

[domains.infra.actions.set-password]
description = "Cambiar contraseña del servicio"
level = "admin"
timeout = 30
execute = "sudo /usr/local/bin/set-password.sh {password}"

[[domains.infra.actions.set-password.args]]
name = "password"
type = "string"
sensitive = true    # Enmascarado en logs cuando mask_sensitive = true
```

### Tipos de argumentos

| Tipo | Descripción | Validación |
|------|-------------|------------|
| `string` | Texto libre | Máx. 256 caracteres |
| `enum` | Valor de una lista | Debe coincidir con un valor en `values` |

### Marcadores en `execute`

- `{domain}`: reemplazado por el nombre del dominio (siempre disponible)
- `{arg_name}`: reemplazado por el valor del argumento correspondiente

## Despliegue

### 1. Shell de inicio de sesión (`/etc/passwd`)

```bash
# Crear la cuenta de servicio
sudo useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

El programa será invocado directamente por `sshd` como shell de inicio de sesión.

### 2. Claves SSH con `authorized_keys`

```
# ~forge-runner/.ssh/authorized_keys

# Clave del runner CI (nivel ops)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-key

# Clave de monitoreo (nivel solo lectura)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitor-key

# Clave de administrador (nivel admin)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

La opción `command=` fuerza la ejecución de ssh-frontiere con el `--level` elegido, independientemente del comando enviado por el cliente. La opción `restrict` deshabilita el reenvío de puertos, reenvío de agente, PTY y X11.

### 3. Sudoers (capa 3)

```
# /etc/sudoers.d/ssh-frontiere
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/set-password.sh
```

Solo los comandos listados en la lista blanca TOML **y** autorizados en sudoers pueden ejecutarse con privilegios elevados.

## Protocolo de cabeceras

SSH Frontière utiliza un protocolo de texto sobre stdin/stdout con 4 prefijos (ADR 0006).

### Prefijos

| Prefijo | Rol | Dirección |
|---------|-----|-----------|
| `+` | **Configurar**: directivas (`capabilities`, `challenge`, `auth`, `session`) | bidireccional |
| `#` | **Comentario**: información, banner, mensajes | bidireccional |
| `$` | **Comando**: comando a ejecutar | cliente → servidor |
| `>` | **Responder**: respuesta JSON | servidor → cliente |

### Flujo de conexión

```
CLIENTE                             SERVIDOR
  |                                    |
  |  <-- banner + capabilities -----  |   # ssh-frontiere 3.0.0
  |  <-- nonce de desafío ----------  |   + capabilities rbac, session, help
  |                                    |   + challenge nonce=a1b2c3...
  |                                    |   # type "help" for available commands
  |                                    |
  |  --- +auth (opcional) -------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (opcional) ----->   |   + session keepalive
  |  --- # comentario (opt.) ----->   |   # client-id: forgejo-runner-12
  |  --- línea vacía ------------->   |   (fin de cabeceras)
  |                                    |
  |  --- dominio acción [args] ---->  |   forgejo backup-config
  |  --- . ------------------------>  |   . (fin del bloque de comando)
  |  <-- >> stdout (streaming) -----  |   >> Backup completed
  |  <-- >>> respuesta JSON --------  |   >>> {"command":"forgejo backup-config","status_code":0,...}
  |                                    |
```

### Respuesta JSON (4 campos)

Cada comando produce una respuesta `>>>` con un objeto JSON:

```json
{
  "command": "forgejo backup-config",
  "status_code": 0,
  "status_message": "executed",
  "stdout": null,
  "stderr": null
}
```

- `stdout`/`stderr` = `null`: la salida fue transmitida via prefijos `>>` / `>>!`
- `status_code` = 0: éxito (código de salida del proceso hijo en passthrough)

### Códigos de salida

| Código | Significado |
|--------|-------------|
| 0 | Éxito |
| 1-127 | Código de salida del comando hijo (passthrough) |
| 128 | Comando rechazado |
| 129 | Error de configuración |
| 130 | Timeout |
| 131 | Nivel RBAC insuficiente |
| 132 | Error de protocolo |
| 133 | Body stdin cerrado prematuramente |

## Ejemplos concretos

### Modo one-shot

```bash
# Pipe simple:
{
  echo "infra healthcheck"
  echo "."
} | ssh forge-runner@server
```

### Modo sesión (keepalive)

El modo sesión permite enviar múltiples comandos en una sola conexión SSH:

```bash
{
  echo "+ session keepalive"
  echo "infra healthcheck"
  echo "."
  echo "forgejo backup-config"
  echo "."
  echo "exit"
  echo "."
} | ssh forge-runner@server
```

El servidor responde con una línea JSON `>>>` para cada comando.

### Autenticación RBAC (elevación de nivel)

Un cliente con `--level=read` puede elevarse a `ops` o `admin` via challenge-response:

```bash
{
  echo "+ auth token=runner-ci proof=<sha256-hex>"
  echo "forgejo backup-config"    # Requiere ops, autorizado via token
  echo "."
} | ssh forge-runner@server
```

El `proof` es `SHA-256(secret)` cuando `challenge_nonce = false`, o `SHA-256(XOR(secret || nonce, secret))` cuando `challenge_nonce = true`. El nivel efectivo es `max(--level, token.level)`.

### Descubrimiento (help / list)

```bash
# Lista completa de comandos accesibles
{ echo "help"; echo "."; } | ssh forge-runner@server

# Detalles del dominio
{ echo "help forgejo"; echo "."; } | ssh forge-runner@server

# Lista corta (dominio + acción + descripción, JSON)
{ echo "list"; echo "."; } | ssh forge-runner@server
```

Los comandos `help` y `list` solo muestran acciones accesibles en el nivel efectivo del cliente.

## Seguridad

### Tres capas de defensa en profundidad

| Capa | Mecanismo | Protección |
|------|-----------|------------|
| 1 | `command=` + `restrict` en `authorized_keys` | Fuerza `--level`, bloquea forwarding/PTY |
| 2 | `ssh-frontiere` (shell de inicio de sesión) | Valida el comando contra la lista blanca TOML |
| 3 | Lista blanca `sudo` en sudoers | Restringe comandos del sistema con privilegios |

Incluso si un atacante elude la capa 1 (clave comprometida), la capa 2 bloquea cualquier comando fuera de la lista blanca. La capa 3 limita los privilegios del sistema.

### Analizador gramatical, no lista negra

**ssh-frontiere no es una shell.** La seguridad se basa en un **analizador gramatical**, no en el filtrado de caracteres.

- La gramática esperada es `domain action [args]` — cualquier cosa que no coincida con esta estructura es rechazada
- Los caracteres especiales (`|`, `;`, `&`, `$`, etc.) dentro de comillas son **contenido** del argumento, no sintaxis de shell — son válidos
- No existen "caracteres prohibidos" — existe una gramática, y todo lo que no la respeta es rechazado
- `std::process::Command` ejecuta directamente sin intermediario de shell — la inyección es estructuralmente imposible

### Lo que el programa NUNCA hace

- Invocar una shell (`/bin/bash`, `/bin/sh`)
- Aceptar pipes, redirecciones o encadenamiento (`|`, `>`, `&&`, `;`)
- Ejecutar un comando no listado en la lista blanca
- Proporcionar acceso a un TTY interactivo

### Protecciones adicionales

- **Timeout** por comando con kill del grupo de procesos (SIGTERM luego SIGKILL)
- **Bloqueo** tras N intentos de autenticación fallidos (configurable, por defecto: 3)
- **Bloqueo de IP** opcional via comando externo configurable (`ban_command`)
- **Enmascaramiento** de argumentos sensibles en logs JSON
- **Límites de tamaño** en la salida capturada (stdout, stderr)
- **Nonce anti-replay** regenerado después de cada autenticación de sesión exitosa
- **env_clear()** en procesos hijos (solo se preserva `PATH`)

## Tests

```bash
# Tests unitarios e de integración
make test

# Tests SSH end-to-end (requiere Docker)
make e2e

# Lints (fmt + clippy)
make lint

# Auditoría de seguridad de dependencias
make audit
```

Los tests E2E (`make e2e`) inician un entorno Docker Compose con un servidor y cliente SSH, luego ejecutan escenarios que cubren el protocolo (PRO-*), autenticación (AUT-*), sesiones (SES-*), seguridad (SEC-*), robustez (ROB-*) y logging (LOG-*).

## Contribuir

¡Las contribuciones son bienvenidas! Consulta la [guía de contribución](CONTRIBUTING.md) para más detalles.

## Licencia

Este proyecto se distribuye bajo la [Licencia Pública de la Unión Europea (EUPL-1.2)](LICENSE.md).

Copyright (c) Julien Garderon, 2024-2026
