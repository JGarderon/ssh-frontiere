+++
title = "Tokens e segurança"
description = "Configurar a autenticação RBAC com tokens no SSH-Frontière"
date = 2026-03-24
weight = 3
+++

# Tokens e segurança

SSH-Frontière propõe dois mecanismos de controlo de acesso complementares: o **nível de base** (via `authorized_keys`) e a **elevação por token** (via o protocolo de cabeçalhos).

## Níveis de base via authorized_keys

Cada chave SSH tem um nível de confiança fixo, definido no `authorized_keys`:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

Este nível é o **mínimo garantido**: um cliente com `--level=read` só pode aceder a ações de nível `read`.

## Elevação por token

Um cliente pode elevar-se acima do seu nível de base autenticando-se com um token. O nível efetivo torna-se `max(nível_base, nível_token)`.

### Configurar um token

```toml
[auth]
challenge_nonce = false    # true para o modo anti-replay

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

### Gerar um segredo

```bash
# Gerar um segredo aleatório
head -c 32 /dev/urandom | base64
# Resultado: algo como "dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="

# No config.toml:
# secret = "b64:dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="
```

### Utilizar um token

A autenticação funciona em dois modos conforme a configuração:

**Modo simples** (`challenge_nonce = false`, por predefinição):

1. O cliente calcula o proof: `SHA-256(secret)`
2. O cliente envia o cabeçalho: `+ auth token=runner-ci proof=...`

**Modo nonce** (`challenge_nonce = true`):

1. O servidor envia um nonce no banner: `+> challenge nonce=a1b2c3...`
2. O cliente calcula o proof: `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. O cliente envia o cabeçalho: `+ auth token=runner-ci proof=...`

```bash
# Calcular o proof com o binário auxiliar
# Modo simples (sem nonce):
PROOF=$(proof --secret "meu-segredo")
# Modo nonce:
PROOF=$(proof --secret "meu-segredo" --nonce "a1b2c3...")

# Enviar com autenticação
{
  echo "+ auth token=runner-ci proof=$PROOF"
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@servidor
```

## Tags de visibilidade

Os tags filtram horizontalmente o acesso às ações. Um token com o tag `forgejo` só vê as ações marcadas com `forgejo`, mesmo que tenha o nível `ops`.

```toml
# Token com tags
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]

# Ação com tags
[domains.forgejo.actions.deploy]
description = "Implementação"
level = "ops"
execute = "sudo /usr/local/bin/deploy.sh {domain}"
tags = ["forgejo", "deploy"]
```

Regras de acesso:
- **Ação sem tags**: acessível por todos (se o nível for suficiente)
- **Ação com tags**: acessível se pelo menos um tag for comum com a identidade
- Em sessão, os tags de vários tokens somam-se (união)

## Modo nonce anti-replay

Por predefinição (`challenge_nonce = false`), o proof é um simples `SHA-256(secret)` — sem nonce. Ao ativar `challenge_nonce = true`, o servidor envia um nonce no banner e o proof integra este nonce. O nonce é regenerado após cada autenticação bem-sucedida, o que impede a reprodução de um proof intercetado.

```toml
[auth]
challenge_nonce = true
```

Este modo é recomendado para acessos fora de SSH (TCP direto) ou quando o canal não é cifrado ponta a ponta.

## Proteção contra abusos

| Proteção | Configuração | Predefinição |
|----------|--------------|--------------|
| Lockout após N falhas | `max_auth_failures` | 3 |
| Ban IP | `ban_command` | desativado |
| Timeout de sessão | `timeout_session` | 3600s |

```toml
[global]
max_auth_failures = 3
ban_command = "/usr/sbin/iptables -A INPUT -s {ip} -j DROP"
```

Após 3 falhas de autenticação, a conexão é cortada. Se `ban_command` estiver configurado, o IP de origem é banido.

---

**Seguinte**: [Usar SSH-Frontière com agentes IA](@/guides/agents-ia.md) — configurar um acesso controlado para LLMs.
