+++
title = "Agentes IA"
description = "Usar SSH-Frontière con agentes IA (Claude Code, etc.)"
date = 2026-03-24
weight = 4
+++

# Usar SSH-Frontière con agentes IA

SSH-Frontière fue disenado desde su origen para ser compatible con los agentes IA (LLM). El protocolo estructurado, el descubrimiento automatico y las respuestas JSON lo convierten en un punto de entrada ideal para los agentes que necesitan actuar sobre un servidor.

## Por que SSH-Frontière para los agentes IA?

Los agentes IA (Claude Code, Cursor, GPT, etc.) pueden ejecutar comandos en un servidor via SSH. El problema: sin control, un agente puede ejecutar cualquier cosa.

SSH-Frontière resuelve este problema:

- **Acotar las acciones**: el agente solo puede ejecutar los comandos configurados
- **Niveles de acceso**: un agente en `read` solo puede consultar, no modificar
- **Descubrimiento**: el agente puede solicitar `help` para conocer las acciones disponibles
- **JSON estructurado**: las respuestas son directamente analizables por el agente

## Configuracion para un agente IA

### 1. Clave SSH dedicada

Genere una clave SSH para el agente:

```bash
ssh-keygen -t ed25519 -C "agent-claude" -f ~/.ssh/agent-claude
```

### 2. Nivel de confianza restringido

En `authorized_keys`, asigne un nivel minimo:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent-claude
```

Comience con `read` y eleve si es necesario mediante un token.

### 3. Dominios dedicados

Configure acciones especificas para el agente:

```toml
[domains.agent]
description = "Acciones para agentes IA"

[domains.agent.actions.status]
description = "Estado de los servicios"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-all.sh"

[domains.agent.actions.logs]
description = "Ultimos logs de aplicacion"
level = "read"
timeout = 30
execute = "/usr/local/bin/recent-logs.sh {service}"

[domains.agent.actions.logs.args]
service = { type = "enum", values = ["web", "api", "worker", "database"] }

[domains.agent.actions.restart]
description = "Reiniciar un servicio"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-service.sh {service}"
tags = ["agent-ops"]

[domains.agent.actions.restart.args]
service = { type = "enum", values = ["web", "api", "worker"] }
```

### 4. Token para elevacion (opcional)

Si el agente necesita acceder a acciones `ops`:

```toml
[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "ops"
tags = ["agent-ops"]
```

## Ejemplo con Claude Code (AutoClaude)

Un agente Claude Code en un contenedor AutoClaude puede usar SSH-Frontière para actuar en el servidor anfitrion:

```bash
# El agente descubre los comandos disponibles (JSON via list)
{ echo "list"; echo "."; } | ssh -i /keys/agent-claude agent@servidor

# El agente verifica el estado de los servicios
{ echo "agent status"; echo "."; } | ssh -i /keys/agent-claude agent@servidor

# El agente lee los logs de un servicio
{ echo "agent logs service=api"; echo "."; } | ssh -i /keys/agent-claude agent@servidor
```

La salida se envia en streaming (`>>`), luego la respuesta JSON final (`>>>`):

```
>> web: running
>> api: running
>> worker: stopped
>>> {"command":"agent status","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

El agente puede analizar las lineas `>>` (salida estandar en streaming), detectar que `worker` esta detenido y decidir actuar en consecuencia. La respuesta `>>>` confirma el codigo de retorno.

## Modo sesion

Para evitar abrir una conexion SSH por comando, el agente puede usar el modo sesion:

```bash
{
  echo "+ auth token=agent-claude proof=..."
  echo "+ session keepalive"
  echo "agent status"
  echo "."
  echo "agent logs service=worker"
  echo "."
  echo "."   # bloque vacio = fin de sesion
} | ssh -i /keys/agent-claude agent@servidor
```

Cada comando va seguido de `.` (fin de bloque). Un `.` sin comando previo senala el fin de sesion. El modo sesion permite enviar varios comandos en una sola conexion SSH, con un timeout global configurable (`timeout_session`).

## Buenas practicas

1. **Principio del menor privilegio**: comience con `read`, eleve por token unicamente si es necesario
2. **Acciones atomicas**: cada accion hace una sola cosa. El agente compone las acciones entre si
3. **Nombres explicitos**: los nombres de dominios y acciones son visibles por `help` — hagalos comprensibles
4. **Tags de visibilidad**: aisle las acciones del agente con tags dedicados
5. **Limites de salida**: configure `max_stdout_chars` para evitar que el agente reciba volumenes excesivos
6. **Logs**: supervise los logs para detectar usos anomalos

---

**Siguiente**: [Integracion CI/CD](@/guides/ci-cd.md) — automatizar los despliegues via SSH-Frontière.
