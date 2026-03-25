+++
title = "SSH-Frontière"
description = "Shell de inicio SSH restringido en Rust — control declarativo de conexiones entrantes"
sort_by = "weight"

[extra]
framed = true
+++

# SSH-Frontière

**Shell de inicio SSH restringido en Rust** — un punto de entrada unico y seguro para todas las conexiones SSH entrantes.

SSH-Frontière reemplaza el shell por defecto de una cuenta Unix (`/bin/bash`) por un programa que **valida cada comando** contra una configuracion declarativa en TOML, antes de ejecutarlo.

[![GitHub](https://img.shields.io/badge/GitHub-Repositorio_open--source-181717?logo=github&logoColor=white&style=for-the-badge)](https://github.com/JGarderon/ssh-frontiere)

---

## Por que SSH-Frontière?

**Seguridad por defecto** — Ningun comando se ejecuta sin estar explicitamente autorizado. Deny by default, sin shell, sin posibilidad de inyeccion.

**Simple de desplegar** — Un binario estatico de ~1 Mo, un archivo TOML, una linea en `/etc/passwd`. Sin daemon, sin servicio que gestionar.

**Flexible** — Tres niveles de acceso (read, ops, admin), tags de visibilidad, un protocolo de cabeceras estructurado. Compatible con agentes IA, runners CI/CD y scripts de mantenimiento.

**Auditable** — Cada comando ejecutado o rechazado se registra en JSON estructurado. 399 tests cargo + 72 escenarios E2E SSH.

---

## Casos de uso

- **Runners CI/CD** (Forgejo Actions, GitHub Actions): despliegues, backups, healthchecks via SSH
- **Agentes IA** (Claude Code, etc.): acceso controlado a recursos del servidor con niveles de confianza
- **Mantenimiento automatizado**: scripts de backup, monitorizacion, notificacion

---

## En resumen

| | |
|---|---|
| **Lenguaje** | Rust (binario estatico musl, ~1 Mo) |
| **Licencia** | [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) — Licencia Publica de la Union Europea |
| **Tests** | 399 cargo + 72 E2E SSH + 9 harnesses fuzz |
| **Dependencias** | 3 crates directas (`serde`, `serde_json`, `toml`) |
| **Configuracion** | TOML declarativo |
| **Protocolo** | Cabeceras de texto sobre stdin/stdout, respuestas JSON |

---

## Para comenzar

- [Descubrir SSH-Frontière](@/presentation.md) — que es, que hace, por que existe
- [Instalacion](@/installation/_index.md) — compilar, configurar, desplegar
- [Guias](@/guides/_index.md) — tutoriales paso a paso
- [Seguridad](@/securite.md) — modelo de seguridad y garantias
- [Arquitectura](@/architecture.md) — diseno tecnico
- [Alternativas](@/alternatives.md) — comparacion con otras soluciones
- [FAQ](@/faq.md) — preguntas frecuentes
- [Contribuir](@/contribuer.md) — participar en el proyecto
