+++
title = "Requisitos previos"
description = "Lo que necesita para instalar SSH-Frontière"
date = 2026-03-24
weight = 1
+++

# Requisitos previos

## Servidor destino

| Elemento | Detalle |
|----------|---------|
| Sistema | Linux x86_64 |
| Acceso SSH | `sshd` funcional |
| Cuenta de servicio | Un usuario dedicado (ej.: `forge-runner`) |
| Cuenta admin de respaldo | Una cuenta con `/bin/bash` (nunca se modificara) |
| Acceso a consola | IPMI, KVM o consola cloud — en caso de bloqueo SSH |

**Importante**: mantenga siempre un acceso a consola funcional y una cuenta admin con un shell clasico. Si el shell de inicio SSH-Frontière esta mal configurado, podria perder el acceso SSH a la cuenta de servicio.

## Maquina de compilacion

Para compilar SSH-Frontière desde las fuentes:

| Elemento | Detalle |
|----------|---------|
| Rust | Version 1.70 o superior |
| Target musl | `x86_64-unknown-linux-musl` (para un binario estatico) |
| `make` | Opcional, para los atajos del Makefile |

### Instalar el target musl

```bash
rustup target add x86_64-unknown-linux-musl
```

## Alternativa: binario precompilado

Si no desea compilar, puede descargar el binario estatico desde las [releases del proyecto](https://github.com/nothus-forge/ssh-frontiere/releases). El binario no tiene ninguna dependencia del sistema.

---

**Siguiente**: [Compilacion desde las fuentes](@/installation/compilation.md)
