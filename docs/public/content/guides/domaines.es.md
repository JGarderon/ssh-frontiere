+++
title = "Dominios y acciones"
description = "Configurar dominios y acciones en SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Configurar dominios y acciones

Un **dominio** es un perimetro funcional (una aplicacion, un servicio, una categoria de operaciones). Cada dominio contiene **acciones**: los comandos autorizados.

## Anadir un dominio de despliegue

```toml
[domains.miapp]
description = "Aplicacion web principal"

[domains.miapp.actions.deploy]
description = "Desplegar una version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy-miapp.sh {tag}"

[domains.miapp.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.miapp.actions.status]
description = "Verificar el estado del servicio"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-miapp.sh"

[domains.miapp.actions.restart]
description = "Reiniciar el servicio"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-miapp.sh"
```

Uso:

```bash
# Desplegar la version estable
{ echo "miapp deploy version=stable"; echo "."; } | ssh ops@servidor

# Verificar el estado
{ echo "miapp status"; echo "."; } | ssh monitoring@servidor

# Reiniciar
{ echo "miapp restart"; echo "."; } | ssh ops@servidor
```

## Anadir un dominio de backup

```toml
[domains.backup]
description = "Backups automatizados"

[domains.backup.actions.full]
description = "Backup completo"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"

[domains.backup.actions.config-only]
description = "Backup de la configuracion"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
```

## Anadir un dominio de notificacion

```toml
[domains.notify]
description = "Notificaciones"

[domains.notify.actions.slack]
description = "Enviar una notificacion Slack"
level = "ops"
timeout = 30
execute = "/usr/local/bin/notify-slack.sh {channel} {message}"

[domains.notify.actions.slack.args]
channel = { type = "enum", values = ["general", "ops", "alerts"], default = "ops" }
message = { free = true }
```

El argumento `message` se declara con `free = true`: acepta cualquier valor textual.

```bash
{ echo 'notify slack channel=ops message="Despliegue completado"'; echo "."; } | ssh ops@servidor
```

## Anadir un dominio de mantenimiento

```toml
[domains.infra]
description = "Infraestructura del servidor"

[domains.infra.actions.healthcheck]
description = "Verificacion de salud de los servicios"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.disk-usage]
description = "Espacio en disco"
level = "read"
timeout = 10
execute = "/usr/bin/df -h"

[domains.infra.actions.logs]
description = "Ultimos logs del sistema"
level = "ops"
timeout = 30
execute = "sudo /usr/bin/journalctl -n 100 --no-pager"
```

## Checklist tras anadir una accion

1. Verificar la sintaxis TOML (un error = fail-fast, codigo 129)
2. Crear el script de ejecucion si es necesario
3. Anadir en sudoers si el comando usa `sudo`
4. Probar con `ssh user@servidor` desde otro terminal
5. Verificar los logs en `/var/log/ssh-frontiere/commands.json`

## Descubrimiento

Los comandos `help` y `list` permiten ver las acciones disponibles:

```bash
# Lista completa con descripciones (texto legible via #>)
{ echo "help"; echo "."; } | ssh user@servidor

# Detalles de un dominio (texto legible via #>)
{ echo "help miapp"; echo "."; } | ssh user@servidor

# Lista corta en JSON (dominio + accion)
{ echo "list"; echo "."; } | ssh user@servidor
```

`help` devuelve texto legible (prefijo `#>`). `list` devuelve JSON estructurado — mas adecuado para el analisis automatico. Ambos muestran unicamente las acciones accesibles al nivel efectivo del cliente.

---

**Siguiente**: [Tokens y niveles de seguridad](@/guides/tokens.md) — controlar quien puede hacer que.
