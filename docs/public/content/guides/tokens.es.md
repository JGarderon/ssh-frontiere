+++
title = "Tokens y seguridad"
description = "Configurar la autenticacion RBAC con tokens en SSH-Frontière"
date = 2026-03-24
weight = 3
+++

# Tokens y seguridad

SSH-Frontière propone dos mecanismos de control de acceso complementarios: el **nivel base** (via `authorized_keys`) y la **elevacion por token** (via el protocolo de cabeceras).

## Niveles base via authorized_keys

Cada clave SSH tiene un nivel de confianza fijo, definido en `authorized_keys`:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

Este nivel es el **minimo garantizado**: un cliente con `--level=read` solo puede acceder a las acciones de nivel `read`.

## Elevacion por token

Un cliente puede elevarse por encima de su nivel base autenticandose con un token. El nivel efectivo se convierte en `max(nivel_base, nivel_token)`.

### Configurar un token

```toml
[auth]
challenge_nonce = false    # true para el modo anti-replay

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

### Generar un secreto

```bash
# Generar un secreto aleatorio
head -c 32 /dev/urandom | base64
# Resultado: algo como "dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="

# En config.toml:
# secret = "b64:dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="
```

### Usar un token

La autenticacion funciona en dos modos segun la configuracion:

**Modo simple** (`challenge_nonce = false`, por defecto):

1. El cliente calcula el proof: `SHA-256(secret)`
2. El cliente envia la cabecera: `+ auth token=runner-ci proof=...`

**Modo nonce** (`challenge_nonce = true`):

1. El servidor envia un nonce en el banner: `+> challenge nonce=a1b2c3...`
2. El cliente calcula el proof: `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. El cliente envia la cabecera: `+ auth token=runner-ci proof=...`

```bash
# Calcular el proof con el binario auxiliar
# Modo simple (sin nonce):
PROOF=$(proof --secret "mi-secreto")
# Modo nonce:
PROOF=$(proof --secret "mi-secreto" --nonce "a1b2c3...")

# Enviar con autenticacion
{
  echo "+ auth token=runner-ci proof=$PROOF"
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@servidor
```

## Tags de visibilidad

Los tags filtran horizontalmente el acceso a las acciones. Un token con el tag `forgejo` solo ve las acciones etiquetadas con `forgejo`, incluso si tiene el nivel `ops`.

```toml
# Token con tags
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]

# Accion con tags
[domains.forgejo.actions.deploy]
description = "Despliegue"
level = "ops"
execute = "sudo /usr/local/bin/deploy.sh {domain}"
tags = ["forgejo", "deploy"]
```

Reglas de acceso:
- **Accion sin tags**: accesible para todos (si el nivel es suficiente)
- **Accion con tags**: accesible si al menos un tag es comun con la identidad
- En sesion, los tags de varios tokens se suman (union)

## Modo nonce anti-replay

Por defecto (`challenge_nonce = false`), el proof es un simple `SHA-256(secret)` — sin nonce. Al activar `challenge_nonce = true`, el servidor envia un nonce en el banner y el proof integra ese nonce. El nonce se regenera despues de cada autenticacion exitosa, lo que impide la reutilizacion de un proof interceptado.

```toml
[auth]
challenge_nonce = true
```

Este modo se recomienda para los accesos fuera de SSH (TCP directo) o cuando el canal no esta cifrado de extremo a extremo.

## Proteccion contra abusos

| Proteccion | Configuracion | Defecto |
|------------|---------------|---------|
| Lockout tras N fallos | `max_auth_failures` | 3 |
| Ban de IP | `ban_command` | desactivado |
| Timeout de sesion | `timeout_session` | 3600s |

```toml
[global]
max_auth_failures = 3
ban_command = "/usr/sbin/iptables -A INPUT -s {ip} -j DROP"
```

Tras 3 fallos de autenticacion, la conexion se corta. Si `ban_command` esta configurado, la IP de origen se banea.

---

**Siguiente**: [Usar SSH-Frontière con agentes IA](@/guides/agents-ia.md) — configurar un acceso controlado para LLM.
