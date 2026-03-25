+++
title = "Compilação"
description = "Compilar SSH-Frontière a partir do código-fonte"
date = 2026-03-24
weight = 2
+++

# Compilação a partir do código-fonte

## Compilação release

```bash
# Via make (recomendado)
make release

# Ou diretamente com cargo
cargo build --release --target x86_64-unknown-linux-musl
```

O binário resultante encontra-se em:

```
target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

É um **binário estático** de aproximadamente 1 Mo, sem qualquer dependência de sistema. Pode ser copiado diretamente para qualquer servidor Linux x86_64.

## Verificação

```bash
# Verificar o tipo do binário
file target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ELF 64-bit LSB executable, x86-64, statically linked

# Verificar o tamanho
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ~1-2 Mo
```

## Compilação debug

Para desenvolvimento:

```bash
make build
# ou
cargo build
```

## Testes

Antes de implementar, verifique que os testes passam:

```bash
# Testes unitários e de integração
make test

# Lints (formatação + clippy)
make lint

# Auditoria das dependências
make audit
```

## Binário auxiliar: proof

Um binário auxiliar está incluído para calcular os proofs de autenticação:

```bash
cargo build --release --target x86_64-unknown-linux-musl --bin proof
```

Este binário é útil para testar a autenticação challenge-response sem implementar o cálculo SHA-256 do lado do cliente.

---

**Seguinte**: [Configuração](@/installation/configuration.md) — preparar o ficheiro `config.toml`.
