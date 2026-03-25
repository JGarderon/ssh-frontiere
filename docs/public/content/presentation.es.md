+++
title = "Presentacion"
description = "Descubrir SSH-Frontière: que es, por que existe, como funciona"
date = 2026-03-24
weight = 1
+++

# Presentacion de SSH-Frontière

## El problema

En un servidor Linux, las cuentas de servicio SSH (runners CI, agentes IA, scripts de mantenimiento) utilizan generalmente `/bin/bash` como shell de inicio de sesion. Esto plantea varios problemas:

- **Ningun control**: el cliente SSH puede ejecutar cualquier comando
- **Sin auditoria**: los comandos ejecutados no se registran de forma estructurada
- **Sin granularidad**: un script que necesita leer un estado tiene los mismos permisos que un script de despliegue

Las soluciones clasicas (`authorized_keys` con `command=`, scripts wrapper en bash, bastiones SSH) tienen cada una sus limitaciones: fragiles, dificiles de auditar o sobredimensionadas para la necesidad.

## Que hace SSH-Frontière

SSH-Frontière es un **shell de inicio de reemplazo**. Se situa entre `sshd` y los comandos del sistema:

```
Cliente SSH
    |
    v
sshd (autenticacion por clave)
    |
    v
ssh-frontiere (shell de inicio)
    |
    |-- Valida el comando contra la configuracion TOML
    |-- Verifica el nivel de acceso (read / ops / admin)
    |-- Ejecuta el comando autorizado
    +-- Devuelve el resultado en JSON estructurado
```

Cada conexion SSH crea un nuevo proceso `ssh-frontiere` que:

1. Muestra un banner y las capacidades del servidor
2. Lee las cabeceras del cliente (autenticacion, modo sesion)
3. Lee el comando (`dominio accion [argumentos]`, texto plano)
4. Valida contra la whitelist TOML
5. Ejecuta si esta autorizado, rechaza en caso contrario
6. Devuelve una respuesta JSON y termina

El programa es **sincrono y efimero**: sin daemon, sin servicio, sin estado persistente.

## Lo que SSH-Frontière no hace

- **No es un bastion SSH**: sin proxy, sin reenvio de conexiones hacia otros servidores
- **No es un gestor de claves**: la gestion de claves SSH permanece en `authorized_keys` y `sshd`
- **No es un shell**: sin interpretacion de comandos, sin pipe, sin redireccion, sin interactividad
- **No es un daemon**: se ejecuta y termina con cada conexion

## Casos de uso concretos

### Automatizacion CI/CD

Un runner Forgejo Actions despliega una aplicacion via SSH:

```bash
# El runner envia el comando via SSH
{
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@servidor
```

SSH-Frontière verifica que el runner tiene el nivel `admin`, que la accion `deploy` existe en el dominio `forgejo`, que el argumento `version=stable` es un valor autorizado, y luego ejecuta el script de despliegue configurado.

### Agentes IA

Un agente Claude Code actua en un servidor con permisos acotados:

```bash
# El agente descubre los comandos disponibles
{ echo "list"; echo "."; } | ssh agent-ia@servidor

# El agente ejecuta una accion especifica
{ echo "infra healthcheck"; echo "."; } | ssh agent-ia@servidor
```

El agente solo tiene acceso a las acciones de nivel `read` configuradas para el. Los comandos `help` y `list` le permiten descubrir las acciones disponibles y sus parametros — formato JSON, directamente analizable.

### Mantenimiento automatizado

Scripts cron ejecutan backups via SSH:

```bash
# Backup nocturno
{ echo "forgejo backup-config"; echo "."; } | ssh backup@servidor

# Notificacion tras el despliegue
{ echo 'notify send message="Despliegue completado"'; echo "."; } | ssh notify@servidor
```

### Notificaciones

Disparar notificaciones (Slack, Olvid, email) como acciones SSH-Frontière estandar:

```bash
{ echo 'notify slack channel=ops message="Build OK"'; echo "."; } | ssh notify@servidor
```

## Por que SSH-Frontière en vez de...

### ...scripts bash en `authorized_keys`?

La opcion `command=` en `authorized_keys` permite forzar un comando, pero:
- Un solo script por clave — sin granularidad
- Sin validacion de argumentos
- Sin niveles de acceso
- Sin logging estructurado
- El script bash puede contener vulnerabilidades (inyeccion, globbing)

SSH-Frontière ofrece una configuracion declarativa, RBAC, logging JSON y un analizador gramatical que elimina las inyecciones.

### ...un bastion SSH (Teleport, Boundary)?

Los bastiones SSH estan disenados para gestionar el acceso de **personas** a servidores:
- Pesados de desplegar y mantener
- Sobredimensionados para cuentas de servicio
- Modelo de amenaza diferente (usuario interactivo vs script automatizado)

SSH-Frontière es un componente ligero (~1 Mo) disenado para las **cuentas de servicio**: sin sesion interactiva, sin proxy, solo validacion de comandos.

### ...`sudo` solo?

`sudo` controla la elevacion de privilegios, pero:
- No controla lo que el cliente SSH puede *solicitar*
- Sin protocolo estructurado (entradas/salidas JSON)
- Sin logging integrado a nivel del comando SSH

SSH-Frontière y `sudo` son complementarios: SSH-Frontière valida el comando entrante, `sudo` controla los privilegios del sistema. Son la capa 2 y la capa 3 de la defensa en profundidad.

## El valor del producto

SSH-Frontière aporta una **gobernanza declarativa** de los accesos SSH de servicio:

1. **Todo esta en un archivo TOML**: los dominios, las acciones, los argumentos, los niveles de acceso. Sin logica dispersa en scripts.

2. **Despliegue instantaneo**: como toda la configuracion esta centralizada en un unico archivo TOML, desplegar una nueva version es trivial. Cada conexion SSH crea un nuevo proceso que relee la configuracion — los cambios se aplican en cuanto termina la sesion en curso o inmediatamente para cualquier nuevo cliente.

3. **Cero confianza por defecto**: nada se ejecuta sin estar explicitamente configurado. Sin shell, sin posibilidad de inyeccion.

4. **Auditable**: cada intento (autorizado o rechazado) se registra en JSON estructurado con timestamp, comando, argumentos, nivel, resultado.

5. **Compatible con LLM**: los agentes IA pueden descubrir las acciones disponibles mediante `help`/`list`, e interactuar a traves de un protocolo JSON estructurado — sin necesidad de analizar texto libre.

6. **Europeo y open source**: licencia EUPL-1.2, desarrollado en Francia, sin dependencia de un ecosistema propietario.

---

Para ir mas alla: [Instalacion](@/installation/_index.md) | [Arquitectura](@/architecture.md) | [Seguridad](@/securite.md) | [Alternativas](@/alternatives.md)
