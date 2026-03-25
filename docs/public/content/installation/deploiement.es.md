+++
title = "Despliegue"
description = "Poner SSH-Frontière en produccion en un servidor"
date = 2026-03-24
weight = 4
+++

# Despliegue

El despliegue de SSH-Frontière se realiza en 4 pasos: instalar el binario, configurar las claves SSH, modificar el shell de inicio y asegurar con sudoers.

## 1. Instalar el binario

```bash
# Copiar el binario al servidor
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@servidor:/usr/local/bin/

# En el servidor
chmod 755 /usr/local/bin/ssh-frontiere
```

## 2. Instalar la configuracion

```bash
# Crear el directorio
mkdir -p /etc/ssh-frontiere

# Copiar la configuracion
cp config.toml /etc/ssh-frontiere/config.toml

# Asegurar los permisos (la cuenta de servicio debe poder leer la config)
chown root:forge-runner /etc/ssh-frontiere/config.toml
chmod 640 /etc/ssh-frontiere/config.toml

# Crear el directorio de logs
mkdir -p /var/log/ssh-frontiere
chown forge-runner:forge-runner /var/log/ssh-frontiere
chmod 755 /var/log/ssh-frontiere
```

## 3. Crear la cuenta de servicio

```bash
# Crear el usuario con ssh-frontiere como shell de inicio
useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

O, si la cuenta ya existe:

```bash
# Modificar el shell de inicio
chsh -s /usr/local/bin/ssh-frontiere forge-runner
```

**Precaucion**: no cierre su sesion actual hasta que haya verificado que la conexion SSH funciona desde otra sesion.

## 4. Configurar las claves SSH (capa 1)

Edite `~forge-runner/.ssh/authorized_keys`:

```
# Clave runner CI (nivel ops)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci

# Clave monitoring (nivel read solamente)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring

# Clave admin (nivel admin)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-deploy
```

La opcion `command=` fuerza la ejecucion de `ssh-frontiere` con el `--level` elegido, sea cual sea el comando enviado por el cliente. La opcion `restrict` desactiva el reenvio de puertos, el agente de reenvio, el PTY y las X11.

```bash
# Asegurar los permisos
chmod 700 ~forge-runner/.ssh
chmod 600 ~forge-runner/.ssh/authorized_keys
chown -R forge-runner:forge-runner ~forge-runner/.ssh
```

## 5. Configurar sudoers (capa 3)

Cree `/etc/sudoers.d/ssh-frontiere`:

```
# SSH-Frontière: comandos autorizados para la cuenta de servicio
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/healthcheck.sh
```

El comodin `*` es necesario para los scripts que reciben argumentos (ej.: `backup-config.sh forgejo`). Los scripts sin argumentos (como `healthcheck.sh`) no lo necesitan.

Valide la sintaxis:

```bash
visudo -c -f /etc/sudoers.d/ssh-frontiere
```

## 6. Verificar

```bash
# Probar desde otro terminal (no cierre la sesion actual)

# Verificar que se muestran los comandos disponibles
{ echo "help"; echo "."; } | ssh forge-runner@servidor

# Probar un comando
{ echo "infra healthcheck"; echo "."; } | ssh forge-runner@servidor
```

## Defensa en profundidad

Las 3 capas se complementan:

| Capa | Mecanismo | Proteccion |
|------|-----------|------------|
| 1 | `command=` + `restrict` en `authorized_keys` | Fuerza el nivel, bloquea forwarding/PTY |
| 2 | SSH-Frontière (shell de inicio) | Valida contra la whitelist TOML |
| 3 | `sudo` en sudoers | Restringe los comandos del sistema |

Incluso si un atacante compromete una clave SSH, solo puede ejecutar los comandos autorizados en la whitelist. Incluso si elude la capa 2, los privilegios estan limitados por sudoers.

## Rollback

Si algo no funciona, vuelva al shell clasico:

```bash
# Via la consola (IPMI/KVM) u otra cuenta admin
chsh -s /bin/bash forge-runner
```

**Consejo**: haga una copia de seguridad de `/etc/passwd` antes de modificar el shell de inicio.

```bash
cp /etc/passwd /etc/passwd.bak.$(date +%Y%m%d)
```

---

**Siguiente**: [Primer uso](@/guides/premier-usage.md) — su primer comando SSH a traves de SSH-Frontière.
