+++
title = "토큰과 보안"
description = "SSH-Frontière에서 토큰으로 RBAC 인증 구성하기"
date = 2026-03-24
weight = 3
+++

# 토큰과 보안

SSH-Frontière는 두 가지 상호 보완적인 접근 제어 메커니즘을 제공합니다: **기본 수준** (`authorized_keys`를 통한)과 **토큰 상승** (헤더 프로토콜을 통한).

## authorized_keys를 통한 기본 수준

각 SSH 키에는 `authorized_keys`에 정의된 고정 신뢰 수준이 있습니다:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

이 수준이 **보장되는 최소치**입니다: `--level=read`를 가진 클라이언트는 `read` 수준의 액션에만 접근 가능합니다.

## 토큰 상승

클라이언트는 토큰 인증을 통해 기본 수준 이상으로 상승할 수 있습니다. 유효 수준은 `max(기본_수준, 토큰_수준)`이 됩니다.

### 토큰 구성

```toml
[auth]
challenge_nonce = false    # true: 재사용 방지 모드

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

### 시크릿 생성

```bash
# 랜덤 시크릿 생성
head -c 32 /dev/urandom | base64
# 결과: "dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ=" 같은 값

# config.toml에서:
# secret = "b64:dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="
```

### 토큰 사용

인증은 구성에 따라 두 가지 모드로 작동합니다:

**단순 모드** (`challenge_nonce = false`, 기본값):

1. 클라이언트가 proof 계산: `SHA-256(secret)`
2. 클라이언트가 헤더 전송: `+ auth token=runner-ci proof=...`

**논스 모드** (`challenge_nonce = true`):

1. 서버가 배너에서 논스 전송: `+> challenge nonce=a1b2c3...`
2. 클라이언트가 proof 계산: `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. 클라이언트가 헤더 전송: `+ auth token=runner-ci proof=...`

```bash
# 보조 바이너리로 proof 계산
# 단순 모드 (논스 없음):
PROOF=$(proof --secret "mon-secret")
# 논스 모드:
PROOF=$(proof --secret "mon-secret" --nonce "a1b2c3...")

# 인증과 함께 전송
{
  echo "+ auth token=runner-ci proof=$PROOF"
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@serveur
```

## 가시성 태그

태그는 액션 접근을 수평적으로 필터링합니다. `forgejo` 태그를 가진 토큰은 `ops` 수준이어도 `forgejo` 태그가 달린 액션만 볼 수 있습니다.

```toml
# 태그가 있는 토큰
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]

# 태그가 있는 액션
[domains.forgejo.actions.deploy]
description = "Déploiement"
level = "ops"
execute = "sudo /usr/local/bin/deploy.sh {domain}"
tags = ["forgejo", "deploy"]
```

접근 규칙:
- **태그가 없는 액션**: 모두에게 접근 가능 (수준이 충분한 경우)
- **태그가 있는 액션**: ID와 하나 이상의 공통 태그가 있어야 접근 가능
- 세션에서 여러 토큰의 태그는 합산됨 (합집합)

## 재사용 방지 논스 모드

기본적으로(`challenge_nonce = false`), proof는 단순한 `SHA-256(secret)`이며 논스가 없습니다. `challenge_nonce = true`를 활성화하면, 서버가 배너에서 논스를 보내고 proof에 이 논스가 포함됩니다. 논스는 인증 성공 후 재생성되어 가로챈 proof의 재사용을 방지합니다.

```toml
[auth]
challenge_nonce = true
```

이 모드는 SSH 외부(TCP 직접 연결) 접근이나 채널이 엔드 투 엔드 암호화되지 않은 경우 권장됩니다.

## 남용 방지

| 보호 | 구성 | 기본값 |
|------|------|--------|
| N회 실패 후 잠금 | `max_auth_failures` | 3 |
| IP 차단 | `ban_command` | 비활성화 |
| 세션 타임아웃 | `timeout_session` | 3600초 |

```toml
[global]
max_auth_failures = 3
ban_command = "/usr/sbin/iptables -A INPUT -s {ip} -j DROP"
```

3회 인증 실패 후 연결이 끊어집니다. `ban_command`가 구성되어 있으면 소스 IP가 차단됩니다.

---

**다음**: [AI 에이전트와 SSH-Frontière 사용](@/guides/agents-ia.md) — LLM을 위한 접근 제어 구성.
