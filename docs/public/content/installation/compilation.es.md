+++
title = "Compilacion"
description = "Compilar SSH-Frontière desde las fuentes"
date = 2026-03-24
weight = 2
+++

# Compilacion desde las fuentes

## Compilacion release

```bash
# Via make (recomendado)
make release

# O directamente con cargo
cargo build --release --target x86_64-unknown-linux-musl
```

El binario resultante se encuentra en:

```
target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

Es un **binario estatico** de aproximadamente 1 Mo, sin ninguna dependencia del sistema. Se puede copiar directamente a cualquier servidor Linux x86_64.

## Verificacion

```bash
# Verificar el tipo del binario
file target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ELF 64-bit LSB executable, x86-64, statically linked

# Verificar el tamano
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ~1-2 Mo
```

## Compilacion debug

Para desarrollo:

```bash
make build
# o
cargo build
```

## Tests

Antes de desplegar, verifique que los tests pasen:

```bash
# Tests unitarios y de integracion
make test

# Lints (formateo + clippy)
make lint

# Auditoria de dependencias
make audit
```

## Binario auxiliar: proof

Se incluye un binario auxiliar para calcular los proofs de autenticacion:

```bash
cargo build --release --target x86_64-unknown-linux-musl --bin proof
```

Este binario es util para probar la autenticacion challenge-response sin implementar el calculo SHA-256 en el lado del cliente.

---

**Siguiente**: [Configuracion](@/installation/configuration.md) — preparar el archivo `config.toml`.
