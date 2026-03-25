+++
title = "Alternativas"
description = "Comparacion de SSH-Frontière con soluciones existentes de control SSH"
date = 2026-03-24
weight = 4
+++

# Comparacion con las alternativas

SSH-Frontière no es la unica forma de controlar los accesos SSH. Esta pagina compara los enfoques existentes para ayudarle a elegir la solucion adecuada.

## Tabla comparativa

| Criterio | `authorized_keys` `command=` | SSH-Frontière | Teleport | Boundary |
|----------|------------------------------|---------------|----------|----------|
| **Tipo** | Opcion OpenSSH | Shell de inicio | Bastion SSH | Bastion SSH |
| **Objetivo** | Script unico por clave | Cuentas de servicio | Usuarios humanos | Usuarios humanos |
| **Granularidad** | 1 comando por clave | RBAC 3 niveles, dominios, acciones, argumentos | Roles, labels, RBAC | Politicas IAM |
| **Logging** | No estructurado | JSON estructurado por comando | Sesion completa (replay) | Audit trail |
| **Despliegue** | Nativo (OpenSSH) | 1 binario + 1 archivo TOML | Cluster (auth server, proxy, node) | Cluster (controller, workers) |
| **Dependencias** | Ninguna | 0 dependencias del sistema | Base de datos, certificados | Base de datos |
| **Tamano** | — | ~1 Mo (binario estatico) | ~100 Mo | ~100 Mo |
| **Anti-inyeccion** | Responsabilidad del script | Estructural (analizador gramatical) | N/A (sesion interactiva) | N/A (sesion interactiva) |
| **Compatible LLM** | No | Si (JSON, help, descubrimiento) | No | No |
| **Licencia** | OpenSSH (BSD) | EUPL-1.2 | AGPL-3.0 (OSS) / Comercial | BSL 1.1 |

## `authorized_keys` con `command=`

La opcion `command=` en `authorized_keys` permite forzar la ejecucion de un script en cada conexion. Es la solucion mas simple y mas extendida.

### Ventajas

- **Cero instalacion**: funcionalidad nativa de OpenSSH
- **Simple** para un caso de uso unico (una clave = un comando)

### Limitaciones

- **Un solo script por clave**: sin granularidad fina. Para N acciones diferentes, se necesitan N claves o un script bash que analice `$SSH_ORIGINAL_COMMAND`
- **Sin validacion de argumentos**: el script recibe una cadena cruda y debe validarla por si mismo — fuente de inyeccion si esta mal hecho
- **Sin niveles de acceso**: todas las claves tienen los mismos permisos (o hay que codificarlos en el script)
- **Sin logging estructurado**: los logs dependen del script
- **Fragil**: un script bash con validacion de comandos es dificil de asegurar y mantener

### Cuando elegir `command=`

- Necesidad simple: una clave SSH, un comando fijo, sin parametros
- Sin exigencia de auditoria ni RBAC

## Teleport

[Teleport](https://goteleport.com/) es un bastion SSH completo con grabacion de sesiones, SSO, certificados y audit trail.

### Ventajas

- **Grabacion de sesion**: replay completo de cada sesion SSH
- **SSO integrado**: GitHub, OIDC, SAML
- **Certificados**: sin gestion de claves SSH
- **Auditoria completa**: quien se conecto, cuando, desde donde, que se hizo

### Limitaciones

- **Complejo de desplegar**: auth server, proxy, node agent, base de datos, certificados
- **Disenado para humanos**: sesiones interactivas, sin protocolo machine-to-machine
- **Sobredimensionado** para cuentas de servicio: un runner CI no necesita grabacion de sesiones ni SSO
- **Licencia dual**: la version comunitaria (AGPL-3.0) tiene limitaciones funcionales

### Cuando elegir Teleport

- Gestion de acceso de **personas** a un parque de servidores
- Necesidad de grabacion de sesiones y SSO
- Infraestructura con medios para desplegar y mantener un cluster

## HashiCorp Boundary

[Boundary](https://www.boundaryproject.io/) es un proxy de acceso que abstrae los detalles de conexion e integra fuentes de identidad externas.

### Ventajas

- **Abstraccion de infraestructura**: los usuarios se conectan a destinos logicos, no a IPs
- **Integracion IAM**: Active Directory, OIDC, LDAP
- **Inyeccion de credenciales**: los secretos se inyectan dinamicamente, nunca se comparten

### Limitaciones

- **Complejo**: controller, workers, base de datos, integracion IAM
- **Orientado a usuarios humanos**: no disenado para scripts automatizados
- **Licencia BSL 1.1**: restricciones comerciales en la edicion comunitaria
- **Sin control a nivel de comando**: Boundary controla el acceso a un host, no a un comando especifico

### Cuando elegir Boundary

- Gran parque de servidores con gestion de identidad centralizada
- Necesidad de abstraccion de infraestructura (los usuarios no conocen las IPs)
- Equipo con experiencia en HashiCorp (Vault, Terraform, etc.)

## `sudo` solo

`sudo` controla la elevacion de privilegios para los comandos del sistema. A menudo se usa solo para restringir las acciones de una cuenta de servicio.

### Ventajas

- **Nativo**: presente en todos los sistemas Linux
- **Granular**: reglas detalladas por usuario, comando y argumentos

### Limitaciones

- **No controla la entrada SSH**: cualquier comando puede ser **solicitado** via SSH, aunque `sudo` bloquee la elevacion
- **Sin protocolo**: sin respuesta estructurada, sin logging JSON integrado
- **Configuracion compleja**: las reglas de sudoers se vuelven dificiles de mantener con numerosos comandos

### Cuando elegir `sudo` solo

- Entorno simple donde el riesgo es bajo
- La entrada SSH ya esta controlada por otro mecanismo (bastion, VPN)

## Cuando elegir SSH-Frontière

SSH-Frontière esta disenado para un **caso de uso preciso**: controlar lo que las cuentas de servicio (no los humanos) pueden hacer via SSH.

Elija SSH-Frontière si:

- Sus conexiones SSH son **scripts automatizados** (CI/CD, agentes IA, cron)
- Necesita **granularidad**: dominios, acciones, argumentos, niveles de acceso
- Quiere **logging JSON estructurado** para auditoria y observabilidad
- Quiere un **despliegue simple**: un binario, un archivo TOML
- Necesita **compatibilidad con LLM**: respuestas JSON, descubrimiento via `help`/`list`
- No quiere desplegar ni mantener un cluster (Teleport, Boundary)

No elija SSH-Frontière si:

- Sus usuarios son **humanos** que necesitan sesiones interactivas completas y ricas
- Necesita un **proxy SSH** hacia otros servidores
- Necesita **SSO**
