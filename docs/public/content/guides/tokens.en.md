+++
title = "Tokens and security"
description = "Configure RBAC authentication with tokens in SSH-Frontière"
date = 2026-03-24
weight = 3
+++

# Tokens and security

SSH-Frontière offers two complementary access control mechanisms: the **base level** (via `authorized_keys`) and **token elevation** (via the header protocol).

## Base levels via authorized_keys

Each SSH key has a fixed trust level, defined in `authorized_keys`:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

This level is the **guaranteed minimum**: a client with `--level=read` can only access `read`-level actions.

## Token elevation

A client can elevate above its base level by authenticating with a token. The effective level becomes `max(base_level, token_level)`.

### Configure a token

```toml
[auth]
challenge_nonce = false    # true for anti-replay mode

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

### Generate a secret

```bash
# Generate a random secret
head -c 32 /dev/urandom | base64
# Result: something like "dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="

# In config.toml:
# secret = "b64:dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="
```

### Use a token

Authentication works in two modes depending on the configuration:

**Simple mode** (`challenge_nonce = false`, default):

1. The client computes the proof: `SHA-256(secret)`
2. The client sends the header: `+ auth token=runner-ci proof=...`

**Nonce mode** (`challenge_nonce = true`):

1. The server sends a nonce in the banner: `+> challenge nonce=a1b2c3...`
2. The client computes the proof: `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. The client sends the header: `+ auth token=runner-ci proof=...`

```bash
# Compute the proof with the auxiliary binary
# Simple mode (without nonce):
PROOF=$(proof --secret "my-secret")
# Nonce mode:
PROOF=$(proof --secret "my-secret" --nonce "a1b2c3...")

# Send with authentication
{
  echo "+ auth token=runner-ci proof=$PROOF"
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@server
```

## Visibility tags

Tags filter access to actions horizontally. A token with the `forgejo` tag only sees actions tagged `forgejo`, even if it has the `ops` level.

```toml
# Token with tags
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]

# Action with tags
[domains.forgejo.actions.deploy]
description = "Deployment"
level = "ops"
execute = "sudo /usr/local/bin/deploy.sh {domain}"
tags = ["forgejo", "deploy"]
```

Access rules:
- **Action without tags**: accessible to everyone (if the level is sufficient)
- **Action with tags**: accessible if at least one tag is shared with the identity
- In session, tags from multiple tokens are combined (union)

## Anti-replay nonce mode

By default (`challenge_nonce = false`), the proof is a simple `SHA-256(secret)` — no nonce. When `challenge_nonce = true` is enabled, the server sends a nonce in the banner and the proof incorporates this nonce. The nonce is regenerated after each successful authentication, preventing replay of an intercepted proof.

```toml
[auth]
challenge_nonce = true
```

This mode is recommended for access outside SSH (direct TCP) or when the channel is not end-to-end encrypted.

## Abuse protection

| Protection | Configuration | Default |
|------------|---------------|---------|
| Lockout after N failures | `max_auth_failures` | 3 |
| IP ban | `ban_command` | disabled |
| Session timeout | `timeout_session` | 3600s |

```toml
[global]
max_auth_failures = 3
ban_command = "/usr/sbin/iptables -A INPUT -s {ip} -j DROP"
```

After 3 failed authentication attempts, the connection is closed. If `ban_command` is configured, the source IP is banned.

---

**Next**: [Using SSH-Frontière with AI agents](@/guides/agents-ia.md) — configure controlled access for LLMs.
