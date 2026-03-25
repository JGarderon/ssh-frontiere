+++
title = "Pré-requisitos"
description = "O que precisa para instalar SSH-Frontière"
date = 2026-03-24
weight = 1
+++

# Pré-requisitos

## Servidor de destino

| Elemento | Detalhe |
|----------|---------|
| Sistema | Linux x86_64 |
| Acesso SSH | `sshd` funcional |
| Conta de serviço | Um utilizador dedicado (ex.: `forge-runner`) |
| Conta admin de socorro | Uma conta com `/bin/bash` (nunca será modificada) |
| Acesso à consola | IPMI, KVM ou consola cloud — em caso de lockout SSH |

**Importante**: mantenha sempre um acesso à consola funcional e uma conta admin com um shell clássico. Se o login shell SSH-Frontière estiver mal configurado, poderá perder o acesso SSH à conta de serviço.

## Máquina de build

Para compilar SSH-Frontière a partir do código-fonte:

| Elemento | Detalhe |
|----------|---------|
| Rust | Versão 1.70 ou superior |
| Alvo musl | `x86_64-unknown-linux-musl` (para um binário estático) |
| `make` | Opcional, para os atalhos do Makefile |

### Instalar o alvo musl

```bash
rustup target add x86_64-unknown-linux-musl
```

## Alternativa: binário pré-compilado

Se não pretende compilar, pode descarregar o binário estático a partir das [releases do projeto](https://github.com/nothus-forge/ssh-frontiere/releases). O binário não tem qualquer dependência de sistema.

---

**Seguinte**: [Compilação a partir do código-fonte](@/installation/compilation.md)
