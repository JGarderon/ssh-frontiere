+++
title = "구성"
description = "SSH-Frontière config.toml 파일 작성하기"
date = 2026-03-24
weight = 3
+++

# 구성

SSH-Frontière는 TOML 파일을 사용하여 도메인, 액션, 접근 수준, 인자 및 인증 토큰을 선언합니다.

## 위치

**기본 경로**: `/etc/ssh-frontiere/config.toml`

**재정의** (우선순위 순):
1. `authorized_keys`의 `command=` 줄에 있는 `--config <path>`
2. 환경 변수 `SSH_FRONTIERE_CONFIG`
3. 기본 경로

**권장 권한**: `root:forge-runner 640` (사용하는 서비스 계정에 맞게 그룹을 조정하세요).

## 파일 구조

```toml
[global]                              # 일반 설정
[domains.<id>]                        # 기능 도메인
  [domains.<id>.actions.<id>]         # 허가된 액션
    [domains.<id>.actions.<id>.args]  # 명명된 인자 (선택사항)
[auth]                                # RBAC 인증 (선택사항)
  [auth.tokens.<id>]                  # 시크릿, 수준 및 태그를 가진 토큰
```

## `[global]` 섹션

| 키 | 유형 | 기본값 | 설명 |
|----|------|--------|------|
| `log_file` | string | **필수** | JSON 로그 파일 경로 |
| `default_timeout` | 정수 | `300` | 기본 타임아웃(초) |
| `max_stdout_chars` | 정수 | `65536` | stdout 제한 (64 Ko) |
| `max_stderr_chars` | 정수 | `16384` | stderr 제한 (16 Ko) |
| `max_output_chars` | 정수 | `131072` | 전체 하드 리밋 (128 Ko) |
| `max_stream_bytes` | 정수 | `10485760` | 스트리밍 볼륨 제한 (10 Mo) |
| `timeout_session` | 정수 | `3600` | 세션 keepalive 타임아웃 |
| `max_auth_failures` | 정수 | `3` | 잠금 전 인증 시도 횟수 |
| `ban_command` | string | `""` | IP 차단 명령 (플레이스홀더 `{ip}`) |
| `log_comments` | bool | `false` | 클라이언트의 `#` 줄을 기록 |
| `expose_session_id` | bool | `false` | 배너에 세션 UUID 표시 |

`log_level`, `default_level`, `mask_sensitive` 키는 이전 구성과의 호환성을 위해 파서에서 수용하지만, 더 이상 사용되지 않습니다.

## `[domains]` 섹션

**도메인**은 기능적 영역(예: `forgejo`, `infra`, `notify`)입니다. 각 도메인은 허가된 **액션**을 포함합니다.

```toml
[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Sauvegarde la configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
max_body_size = 65536       # body 제한 (64 Ko, 선택사항)
```

각 액션은 다음 키를 받습니다: `description` (필수), `level` (필수), `execute` (필수), `timeout` (선택사항, 전역 설정 재정의), `tags` (선택사항), `max_body_size` (선택사항, 기본값 65536바이트 — `+body` 프로토콜용 제한).

### 신뢰 수준

엄격한 계층 구조: `read` < `ops` < `admin`

| 수준 | 용도 |
|------|------|
| `read` | 조회: healthcheck, status, list |
| `ops` | 일반 운영: backup, deploy, restart |
| `admin` | 모든 액션 + 관리 |

### 인자

인자는 TOML 딕셔너리로 선언됩니다:

```toml
[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
```

| 필드 | 유형 | 설명 |
|------|------|------|
| `type` | string | `"enum"` 또는 `"string"` |
| `values` | 리스트 | 허용된 값 (`enum`용) |
| `default` | string | 기본값 (인자를 선택사항으로 만듦) |
| `sensitive` | bool | `true`이면 로그에서 마스킹 |
| `free` | bool | `true`이면 제약 없이 모든 값 허용 |

### `execute`의 플레이스홀더

| 플레이스홀더 | 설명 |
|--------------|------|
| `{domain}` | 도메인 이름 (항상 사용 가능) |
| `{인자명}` | 해당 인자의 값 |

### 가시성 태그

태그는 액션 접근을 수평적으로 필터링합니다. 태그가 없는 액션은 모두에게 접근 가능합니다. 태그가 있는 액션은 하나 이상의 공통 태그를 가진 ID만 접근 가능합니다.

```toml
[domains.forgejo.actions.deploy]
# ...
tags = ["forgejo", "deploy"]
```

## `[auth]` 섹션 (선택사항)

RBAC 인증은 챌린지-응답을 통한 권한 상승을 가능하게 합니다:

```toml
[auth]
challenge_nonce = false              # true = 재사용 방지 논스 모드

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # base64 인코딩된 시크릿
level = "ops"                               # 부여되는 수준
tags = ["forgejo"]                          # 가시성 태그
```

시크릿은 `b64:` 접두사가 붙고 base64로 인코딩되어야 합니다. 시크릿을 생성하려면:

```bash
echo -n "mon-secret-aleatoire" | base64
# bW9uLXNlY3JldC1hbGVhdG9pcmU=
```

## 로드 시 검증

구성은 매번 로드할 때마다 완전히 검증됩니다(fail-fast). 오류 시 프로그램이 종료 코드 129로 중단됩니다. 검증 항목:

- 올바른 TOML 문법
- 최소 하나의 도메인, 도메인당 최소 하나의 액션
- 각 액션에 유효한 `execute`와 `level`
- `execute`의 플레이스홀더 `{arg}`가 선언된 인자와 일치
- enum 인자에 최소 하나의 허용된 값
- 기본값이 허용된 값 목록에 포함
- `max_stdout_chars`와 `max_stderr_chars` <= `max_output_chars`

## 전체 예제

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 3600
max_auth_failures = 3

[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Forgejo 구성 백업"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"

[domains.forgejo.actions.deploy]
description = "버전 태그로 배포"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.infra]
description = "서버 인프라"

[domains.infra.actions.healthcheck]
description = "서비스 상태 점검"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[auth]
challenge_nonce = false

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

모든 사용 사례를 포함한 자세한 가이드는 저장소의 [전체 구성 가이드](https://github.com/nothus-forge/ssh-frontiere/blob/main/docs/references/configuration.md)를 참조하세요.

---

**다음**: [배포](@/installation/deploiement.md) — 프로덕션 적용.
