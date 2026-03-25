+++
title = "Configuracion"
description = "Escribir el archivo config.toml de SSH-Frontière"
date = 2026-03-24
weight = 3
+++

# Configuracion

SSH-Frontière utiliza un archivo TOML para declarar los dominios, acciones, niveles de acceso, argumentos y tokens de autenticacion.

## Ubicacion

**Ruta por defecto**: `/etc/ssh-frontiere/config.toml`

**Override** (por orden de prioridad):
1. `--config <ruta>` en la linea `command=` de `authorized_keys`
2. Variable de entorno `SSH_FRONTIERE_CONFIG`
3. Ruta por defecto

**Permisos recomendados**: `root:forge-runner 640` (adapte el grupo a la cuenta de servicio utilizada).

## Estructura del archivo

```toml
[global]                              # Parametros generales
[domains.<id>]                        # Dominios funcionales
  [domains.<id>.actions.<id>]         # Acciones autorizadas
    [domains.<id>.actions.<id>.args]  # Argumentos con nombre (opcional)
[auth]                                # Autenticacion RBAC (opcional)
  [auth.tokens.<id>]                  # Tokens con secreto, nivel y tags
```

## Seccion `[global]`

| Clave | Tipo | Defecto | Descripcion |
|-------|------|---------|-------------|
| `log_file` | string | **obligatorio** | Ruta del archivo de log JSON |
| `default_timeout` | entero | `300` | Timeout por defecto en segundos |
| `max_stdout_chars` | entero | `65536` | Limite stdout (64 Ko) |
| `max_stderr_chars` | entero | `16384` | Limite stderr (16 Ko) |
| `max_output_chars` | entero | `131072` | Hard limit global (128 Ko) |
| `max_stream_bytes` | entero | `10485760` | Limite de volumen en streaming (10 Mo) |
| `timeout_session` | entero | `3600` | Timeout de sesion keepalive |
| `max_auth_failures` | entero | `3` | Intentos de auth antes de lockout |
| `ban_command` | string | `""` | Comando de ban de IP (placeholder `{ip}`) |
| `log_comments` | bool | `false` | Registrar las lineas `#` del cliente |
| `expose_session_id` | bool | `false` | Mostrar el UUID de sesion en el banner |

Las claves `log_level`, `default_level` y `mask_sensitive` son aceptadas por el parser para compatibilidad con configuraciones antiguas, pero ya no se utilizan.

## Seccion `[domains]`

Un **dominio** es un perimetro funcional (ej.: `forgejo`, `infra`, `notify`). Cada dominio contiene **acciones** autorizadas.

```toml
[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Respalda la configuracion"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
max_body_size = 65536       # Limite body (64 Ko, opcional)
```

Cada accion acepta las siguientes claves: `description` (obligatorio), `level` (obligatorio), `execute` (obligatorio), `timeout` (opcional, override del global), `tags` (opcional), `max_body_size` (opcional, defecto 65536 bytes — limitado para el protocolo `+body`).

### Niveles de confianza

Jerarquia estricta: `read` < `ops` < `admin`

| Nivel | Uso |
|-------|-----|
| `read` | Consulta: healthcheck, status, list |
| `ops` | Operaciones habituales: backup, deploy, restart |
| `admin` | Todas las acciones + administracion |

### Argumentos

Los argumentos se declaran como un diccionario TOML:

```toml
[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
```

| Campo | Tipo | Descripcion |
|-------|------|-------------|
| `type` | string | `"enum"` o `"string"` |
| `values` | lista | Valores autorizados (para `enum`) |
| `default` | string | Valor por defecto (hace el argumento opcional) |
| `sensitive` | bool | Si `true`, se enmascara en los logs |
| `free` | bool | Si `true`, acepta cualquier valor sin restriccion |

### Placeholders en `execute`

| Placeholder | Descripcion |
|-------------|-------------|
| `{domain}` | Nombre del dominio (siempre disponible) |
| `{nombre_arg}` | Valor del argumento correspondiente |

### Tags de visibilidad

Los tags filtran horizontalmente el acceso a las acciones. Una accion sin tags es accesible para todos. Una accion con tags solo es accesible para las identidades cuyos tags tengan al menos un tag en comun.

```toml
[domains.forgejo.actions.deploy]
# ...
tags = ["forgejo", "deploy"]
```

## Seccion `[auth]` (opcional)

La autenticacion RBAC permite la elevacion de privilegios mediante challenge-response:

```toml
[auth]
challenge_nonce = false              # true = modo nonce anti-replay

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Secreto en base64
level = "ops"                               # Nivel otorgado
tags = ["forgejo"]                          # Tags de visibilidad
```

El secreto debe tener el prefijo `b64:` y estar codificado en base64. Para generar un secreto:

```bash
echo -n "mi-secreto-aleatorio" | base64
# bWktc2VjcmV0by1hbGVhdG9yaW8=
```

## Validacion al cargar

La configuracion se valida integramente en cada carga (fail-fast). En caso de error, el programa se detiene con el codigo 129. Validaciones:

- Sintaxis TOML correcta
- Al menos un dominio, al menos una accion por dominio
- Cada accion tiene un `execute` y un `level` valido
- Los placeholders `{arg}` en `execute` corresponden a los argumentos declarados
- Los argumentos enum tienen al menos un valor autorizado
- Los valores por defecto estan en la lista de valores autorizados
- `max_stdout_chars` y `max_stderr_chars` <= `max_output_chars`

## Ejemplo completo

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 3600
max_auth_failures = 3

[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Respalda la configuracion Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"

[domains.forgejo.actions.deploy]
description = "Despliegue con tag de version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.infra]
description = "Infraestructura del servidor"

[domains.infra.actions.healthcheck]
description = "Verificacion de salud"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[auth]
challenge_nonce = false

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

Para una guia detallada con todos los casos de uso, consulte la [guia de configuracion completa](https://github.com/nothus-forge/ssh-frontiere/blob/main/docs/references/configuration.md) en el repositorio.

---

**Siguiente**: [Despliegue](@/installation/deploiement.md) — poner en produccion.
