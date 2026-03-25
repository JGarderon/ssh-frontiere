+++
title = "SSH-Frontière"
description = "Shell de login SSH restrito em Rust — controlo declarativo das conexões de entrada"
sort_by = "weight"

[extra]
framed = true
+++

# SSH-Frontière

**Shell de login SSH restrito em Rust** — um ponto de entrada único e seguro para todas as conexões SSH de entrada.

SSH-Frontière substitui o shell predefinido de uma conta Unix (`/bin/bash`) por um programa que **valida cada comando** contra uma configuração declarativa em TOML, antes de o executar.

[![GitHub](https://img.shields.io/badge/GitHub-Repositório_open--source-181717?logo=github&logoColor=white&style=for-the-badge)](https://github.com/JGarderon/ssh-frontiere)

---

## Porquê SSH-Frontière?

**Segurança por predefinição** — Nenhum comando é executado sem ser explicitamente autorizado. Deny by default, sem shell, sem injeção possível.

**Simples de implementar** — Um binário estático de ~1 Mo, um ficheiro TOML, uma linha no `/etc/passwd`. Sem daemon, sem serviço a gerir.

**Flexível** — Três níveis de acesso (read, ops, admin), tags de visibilidade, um protocolo de cabeçalhos estruturado. Compatível com agentes IA, runners CI/CD e scripts de manutenção.

**Auditável** — Cada comando executado ou rejeitado é registado em JSON estruturado. 399 testes cargo + 72 cenários E2E SSH.

---

## Casos de utilização

- **Runners CI/CD** (Forgejo Actions, GitHub Actions): implementações, backups, healthchecks via SSH
- **Agentes IA** (Claude Code, etc.): acesso controlado a recursos do servidor com níveis de confiança
- **Manutenção automatizada**: scripts de salvaguarda, de monitorização, de notificação

---

## Em resumo

| | |
|---|---|
| **Linguagem** | Rust (binário estático musl, ~1 Mo) |
| **Licença** | [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) — Licença Pública da União Europeia |
| **Testes** | 399 cargo + 72 E2E SSH + 9 harnesses fuzz |
| **Dependências** | 3 crates diretas (`serde`, `serde_json`, `toml`) |
| **Configuração** | TOML declarativo |
| **Protocolo** | Cabeçalhos de texto em stdin/stdout, respostas JSON |

---

## Para começar

- [Descobrir SSH-Frontière](@/presentation.md) — o que é, o que faz, por que existe
- [Instalação](@/installation/_index.md) — compilar, configurar, implementar
- [Guias](@/guides/_index.md) — tutoriais passo a passo
- [Segurança](@/securite.md) — modelo de segurança e garantias
- [Arquitetura](@/architecture.md) — conceção técnica
- [Alternativas](@/alternatives.md) — comparação com outras soluções
- [FAQ](@/faq.md) — perguntas frequentes
- [Contribuir](@/contribuer.md) — participar no projeto
