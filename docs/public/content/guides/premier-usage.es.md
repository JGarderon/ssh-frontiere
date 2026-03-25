+++
title = "Primer uso"
description = "Instalar SSH-Frontière, configurar un primer dominio y probar"
date = 2026-03-24
weight = 1
+++

# Primer uso

Esta guia le acompana desde la instalacion hasta su primer comando SSH a traves de SSH-Frontière.

## 1. Preparar una configuracion minima

Cree un archivo `config.toml` minimo:

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 60

[domains.test]
description = "Dominio de prueba"

[domains.test.actions.hello]
description = "Comando de prueba que muestra un mensaje"
level = "read"
timeout = 10
execute = "/usr/bin/echo hello from ssh-frontiere"
```

Esta configuracion define un unico dominio `test` con una accion `hello` accesible al nivel `read`.

## 2. Instalar y configurar

Primero debe disponer del binario `ssh-frontiere`. Consulte la [guia de compilacion](@/installation/compilation.md) o descargue un binario precompilado desde la [pagina de releases](https://github.com/nothus-forge/ssh-frontiere/releases).

```bash
# Copiar el binario
sudo cp ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere

# Instalar la configuracion
sudo mkdir -p /etc/ssh-frontiere
sudo cp config.toml /etc/ssh-frontiere/config.toml
sudo chmod 640 /etc/ssh-frontiere/config.toml

# Crear el directorio de logs
sudo mkdir -p /var/log/ssh-frontiere

# Crear la cuenta de servicio
sudo useradd -m -s /usr/local/bin/ssh-frontiere test-user

# Dar acceso de escritura a los logs a la cuenta
sudo chown test-user:test-user /var/log/ssh-frontiere
```

## 3. Configurar la clave SSH

En su maquina cliente:

```bash
# Generar una clave
ssh-keygen -t ed25519 -C "test-key" -f ~/.ssh/test-frontiere
```

En el servidor, anade la clave publica en `~test-user/.ssh/authorized_keys`:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... test-key
```

```bash
# Asegurar los permisos
sudo chmod 700 ~test-user/.ssh
sudo chmod 600 ~test-user/.ssh/authorized_keys
sudo chown -R test-user:test-user ~test-user/.ssh
```

## 4. Primera llamada

```bash
# Descubrir los comandos disponibles
{ echo "help"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@servidor
```

Respuesta esperada (el servidor envia primero el banner, luego la respuesta):

```
#> ssh-frontiere 0.1.0
+> capabilities session, help, body
#> type "help" for available commands
#> ...
>>> {"command":"help","status_code":0,"status_message":"ok","stdout":null,"stderr":null}
```

Las lineas `#>` contienen el texto de ayuda legible. El comando `help` muestra la lista de dominios y acciones accesibles al nivel `read`.

## 5. Ejecutar un comando

```bash
{ echo "test hello"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@servidor
```

Respuesta esperada:

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

La salida del programa (`hello from ssh-frontiere`) se envia en streaming via `>>`, luego la respuesta JSON final via `>>>`. Los campos `stdout` y `stderr` son `null` en el JSON porque la salida fue enviada en streaming.

## 6. Comprender el flujo

Esto es lo que ha ocurrido:

1. El cliente SSH se conecta con la clave `test-frontiere`
2. `sshd` autentica la clave y lee `authorized_keys`
3. La opcion `command=` fuerza la ejecucion de `ssh-frontiere --level=read`
4. SSH-Frontière muestra el banner (`#>`, `+>`) y espera las cabeceras
5. El cliente envia el comando `test hello` (texto plano, sin prefijo) y luego `.` (fin de bloque)
6. SSH-Frontière valida: dominio `test`, accion `hello`, nivel `read` <= `read` requerido
7. SSH-Frontière ejecuta `/usr/bin/echo hello from ssh-frontiere`
8. La salida se envia en streaming (`>>`), luego la respuesta JSON final (`>>>`)

## 7. Probar un rechazo

Intente un comando que no existe:

```bash
{ echo "test inexistente"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@servidor
```

Respuesta:

```
>>> {"command":"test inexistente","status_code":128,"status_message":"rejected: unknown action 'inexistente' in domain 'test'","stdout":null,"stderr":null}
```

`stdout` y `stderr` son `null` porque el comando no fue ejecutado.

## Siguiente paso

Ahora que SSH-Frontière funciona, puede [configurar sus propios dominios y acciones](@/guides/domaines.md).
