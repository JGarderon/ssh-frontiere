# SSH Frontière v3.0.0

**Website**: [pages.nothus.fr/ssh-frontiere](https://pages.nothus.fr/ssh-frontiere/ko/)

[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.md)

Rust로 작성된 제한적 SSH 로그인 셸 — 서버의 모든 SSH 연결을 위한 단일하고 안전한 진입점입니다.

SSH Frontière는 `/etc/passwd`의 기본 셸(`/bin/bash`)을 대체하고 **보안 디스패처**로 기능합니다. 모든 SSH 명령을 TOML 화이트리스트로 검증하고, 3단계 RBAC 접근 제어를 적용하며, stdin/stdout의 헤더 기반 프로토콜을 통해 결과를 구조화된 JSON으로 반환합니다.

## 목적

SSH Frontière는 SSH 서비스 계정을 위한 **보안 컴포넌트**입니다.

- **CI/CD 러너** (Forgejo Actions, GitHub Actions): 컨테이너에서의 인프라 작업
- **AI 에이전트** (Claude Code 등): 신뢰 수준을 갖춘 서버 제어 접근
- **자동화 유지관리**: 백업, 배포, 헬스체크

이 프로그램은 **동기적이고 원샷(one-shot)**입니다. SSH는 각 연결마다 새 프로세스를 생성하고, 디스패처가 검증 및 실행 후 종료합니다. 데몬 없음, 비동기 없음, Tokio 없음.

## 설치

### 사전 요구사항

- Rust 1.70 이상, `x86_64-unknown-linux-musl` 타겟 포함
- `make` (선택사항, 단축키용)

### 컴파일

```bash
# make 경유
make release

# 또는 직접
cargo build --release --target x86_64-unknown-linux-musl
```

생성된 정적 바이너리(`target/x86_64-unknown-linux-musl/release/ssh-frontiere`, 약 1~2 MB)는 시스템 의존성 없이 배포할 수 있습니다.

```bash
sudo cp target/x86_64-unknown-linux-musl/release/ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere
```

## TOML 설정

기본 파일: `/etc/ssh-frontiere/config.toml`.
재정의: `--config <경로>` 또는 환경변수 `SSH_FRONTIERE_CONFIG`.

### 전체 예시

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300          # 기본 타임아웃 (초)
default_level = "read"         # 기본 RBAC 레벨
mask_sensitive = true           # 로그에서 민감한 인수 마스킹
max_stdout_chars = 65536       # 캡처된 stdout 한도
max_stderr_chars = 16384       # 캡처된 stderr 한도
max_output_chars = 131072      # 전체 하드 한도
timeout_session = 3600         # 세션 킵얼라이브 타임아웃 (초)
max_auth_failures = 3          # 잠금 전 인증 시도 횟수
log_comments = false           # 클라이언트 주석 로깅
ban_command = ""               # IP 차단 명령 (예: "/usr/sbin/iptables -A INPUT -s {ip} -j DROP")

# --- RBAC 인증 (선택사항) ---

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # b64: 접두사가 붙은 Base64 인코딩 시크릿
level = "ops"                                # 이 토큰으로 부여되는 레벨

[auth.tokens.admin-deploy]
secret = "b64:c2VjcmV0LWFkbWluLWRlcGxveQ=="
level = "admin"

# --- 도메인 및 액션 ---

[domains.forgejo]
description = "Git 포지 인프라"

[domains.forgejo.actions.backup-config]
description = "Forgejo 설정 백업"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.deploy]
description = "버전 배포"
level = "admin"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {version}"

[[domains.forgejo.actions.deploy.args]]
name = "version"
type = "enum"
values = ["latest", "stable", "canary"]

[domains.infra]
description = "서버 인프라"

[domains.infra.actions.healthcheck]
description = "헬스체크"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"
args = []

[domains.infra.actions.set-password]
description = "서비스 비밀번호 변경"
level = "admin"
timeout = 30
execute = "sudo /usr/local/bin/set-password.sh {password}"

[[domains.infra.actions.set-password.args]]
name = "password"
type = "string"
sensitive = true    # mask_sensitive = true 일 때 로그에서 마스킹됨
```

### 인수 타입

| 타입 | 설명 | 검증 |
|------|------|------|
| `string` | 자유 텍스트 | 최대 256자 |
| `enum` | 목록에서의 값 | `values` 내 값 중 하나와 일치해야 함 |

### `execute` 내 플레이스홀더

- `{domain}`: 도메인 이름으로 대체됨 (항상 사용 가능)
- `{arg_name}`: 해당 인수 값으로 대체됨

## 배포

### 1. 로그인 셸 (`/etc/passwd`)

```bash
# 서비스 계정 생성
sudo useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

프로그램은 `sshd`에 의해 로그인 셸로 직접 호출됩니다.

### 2. `authorized_keys`를 통한 SSH 키 설정

```
# ~forge-runner/.ssh/authorized_keys

# CI 러너 키 (ops 레벨)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-key

# 모니터링 키 (읽기 전용 레벨)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitor-key

# 관리자 키 (admin 레벨)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

`command=` 옵션은 클라이언트가 전송한 명령에 관계없이 선택한 `--level`로 ssh-frontiere 실행을 강제합니다. `restrict` 옵션은 포트 포워딩, 에이전트 포워딩, PTY, X11을 비활성화합니다.

### 3. sudoers (3번째 레이어)

```
# /etc/sudoers.d/ssh-frontiere
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/set-password.sh
```

TOML 화이트리스트에 나열된 **동시에** sudoers에서 승인된 명령만 상승된 권한으로 실행할 수 있습니다.

## 헤더 프로토콜

SSH Frontière는 4개의 접두사를 가진 stdin/stdout 상의 텍스트 프로토콜을 사용합니다 (ADR 0006).

### 접두사

| 접두사 | 역할 | 방향 |
|--------|------|------|
| `+` | **설정**: 지시자 (`capabilities`, `challenge`, `auth`, `session`) | 양방향 |
| `#` | **주석**: 정보, 배너, 메시지 | 양방향 |
| `$` | **명령**: 실행할 명령 | 클라이언트 → 서버 |
| `>` | **응답**: JSON 응답 | 서버 → 클라이언트 |

### 연결 흐름

```
클라이언트                           서버
  |                                    |
  |  <-- 배너 + capabilities -------  |   # ssh-frontiere 3.0.0
  |  <-- 챌린지 nonce -------------  |   + capabilities rbac, session, help
  |                                    |   + challenge nonce=a1b2c3...
  |                                    |   # type "help" for available commands
  |                                    |
  |  --- +auth (선택사항) -------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (선택사항) ----->   |   + session keepalive
  |  --- # 주석 (선택사항) ------->   |   # client-id: forgejo-runner-12
  |  --- 빈 줄 ------------------>   |   (헤더 종료)
  |                                    |
  |  --- 도메인 액션 [인수] ------->  |   forgejo backup-config
  |  --- . ------------------------>  |   . (명령 블록 종료)
  |  <-- >> stdout (스트리밍) ------  |   >> Backup completed
  |  <-- >>> JSON 응답 -----------   |   >>> {"command":"forgejo backup-config","status_code":0,...}
  |                                    |
```

### JSON 응답 (4개 필드)

각 명령은 JSON 객체를 포함하는 `>>>` 응답을 생성합니다.

```json
{
  "command": "forgejo backup-config",
  "status_code": 0,
  "status_message": "executed",
  "stdout": null,
  "stderr": null
}
```

- `stdout`/`stderr` = `null`: 출력이 `>>` / `>>!` 접두사를 통해 스트리밍됨
- `status_code` = 0: 성공 (패스스루에서 자식 프로세스 종료 코드)

### 종료 코드

| 코드 | 의미 |
|------|------|
| 0 | 성공 |
| 1-127 | 자식 명령 종료 코드 (패스스루) |
| 128 | 명령 거부됨 |
| 129 | 설정 오류 |
| 130 | 타임아웃 |
| 131 | RBAC 레벨 부족 |
| 132 | 프로토콜 오류 |
| 133 | body stdin 조기 종료 |

## 구체적인 예시

### 원샷 모드

```bash
# 단순 파이프:
{
  echo "infra healthcheck"
  echo "."
} | ssh forge-runner@server
```

### 세션 모드 (킵얼라이브)

세션 모드는 단일 SSH 연결에서 여러 명령을 전송할 수 있게 합니다.

```bash
{
  echo "+ session keepalive"
  echo "infra healthcheck"
  echo "."
  echo "forgejo backup-config"
  echo "."
  echo "exit"
  echo "."
} | ssh forge-runner@server
```

서버는 각 명령에 대해 `>>>` JSON 줄로 응답합니다.

### RBAC 인증 (레벨 상승)

`--level=read`의 클라이언트는 챌린지-응답으로 `ops` 또는 `admin`으로 상승할 수 있습니다.

```bash
{
  echo "+ auth token=runner-ci proof=<sha256-hex>"
  echo "forgejo backup-config"    # ops 필요, 토큰으로 승인됨
  echo "."
} | ssh forge-runner@server
```

`proof`는 `challenge_nonce = false`일 때 `SHA-256(secret)`, `challenge_nonce = true`일 때 `SHA-256(XOR(secret || nonce, secret))`입니다. 유효 레벨은 `max(--level, token.level)`입니다.

### 디스커버리 (help / list)

```bash
# 접근 가능한 명령의 전체 목록
{ echo "help"; echo "."; } | ssh forge-runner@server

# 도메인 세부 정보
{ echo "help forgejo"; echo "."; } | ssh forge-runner@server

# 요약 목록 (도메인 + 액션 + 설명, JSON)
{ echo "list"; echo "."; } | ssh forge-runner@server
```

`help` 및 `list` 명령은 클라이언트의 유효 레벨에서 접근 가능한 액션만 표시합니다.

## 보안

### 3겹 심층 방어

| 레이어 | 메커니즘 | 보호 내용 |
|--------|---------|---------|
| 1 | `authorized_keys`의 `command=` + `restrict` | `--level` 강제, 포워딩/PTY 차단 |
| 2 | `ssh-frontiere` (로그인 셸) | TOML 화이트리스트로 명령 검증 |
| 3 | sudoers의 `sudo` 화이트리스트 | 권한 있는 시스템 명령 제한 |

공격자가 레이어 1을 우회하더라도(키 침해), 레이어 2가 화이트리스트 외의 모든 명령을 차단합니다. 레이어 3은 시스템 권한을 제한합니다.

### 문법 파서, 블랙리스트가 아님

**ssh-frontiere는 셸이 아닙니다.** 보안은 문자 필터링이 아닌 **문법 파서**에 기반합니다.

- 예상 문법은 `domain action [args]` — 이 구조와 일치하지 않는 것은 모두 거부됩니다
- 따옴표 안의 특수문자(`|`, `;`, `&`, `$` 등)는 인수의 **내용**이지 셸 문법이 아닙니다 — 유효합니다
- "금지된 문자"는 존재하지 않습니다 — 문법이 있고, 그것을 따르지 않는 것이 거부됩니다
- `std::process::Command`는 셸 중개자 없이 직접 실행합니다 — 인젝션은 구조적으로 불가능합니다

### 이 프로그램이 절대 하지 않는 것

- 셸 호출 (`/bin/bash`, `/bin/sh`)
- 파이프, 리다이렉션 또는 체이닝 수용 (`|`, `>`, `&&`, `;`)
- 화이트리스트에 없는 명령 실행
- 대화형 TTY 접근 제공

### 추가 보호 기능

- 프로세스 그룹 킬을 포함한 명령별 **타임아웃** (SIGTERM 후 SIGKILL)
- N번의 인증 실패 후 **잠금** (설정 가능, 기본값: 3회)
- 설정 가능한 외부 명령을 통한 선택적 **IP 차단** (`ban_command`)
- JSON 로그에서 민감한 인수 **마스킹**
- 캡처된 출력(stdout, stderr)의 **크기 제한**
- 각 세션 인증 성공 후 재생성되는 **안티 리플레이 nonce**
- 자식 프로세스의 **env_clear()** (`PATH`만 보존)

## 테스트

```bash
# 유닛 및 통합 테스트
make test

# 엔드투엔드 SSH 테스트 (Docker 필요)
make e2e

# 린트 (fmt + clippy)
make lint

# 의존성 보안 감사
make audit
```

E2E 테스트(`make e2e`)는 SSH 서버와 클라이언트가 있는 Docker Compose 환경을 시작하고 프로토콜(PRO-*), 인증(AUT-*), 세션(SES-*), 보안(SEC-*), 견고성(ROB-*), 로깅(LOG-*)을 아우르는 시나리오를 실행합니다.

## 기여

기여를 환영합니다! 자세한 내용은 [기여 가이드](CONTRIBUTING.md)를 참고하세요.

## 라이선스

이 프로젝트는 [유럽 연합 공중 라이선스(EUPL-1.2)](LICENSE.md) 하에 배포됩니다.

Copyright (c) Julien Garderon, 2024-2026
