+++
title = "배포"
description = "서버에 SSH-Frontière를 프로덕션 배포하기"
date = 2026-03-24
weight = 4
+++

# 배포

SSH-Frontière 배포는 4단계로 이루어집니다: 바이너리 설치, SSH 키 구성, 로그인 셸 변경, sudoers로 보안 강화.

## 1. 바이너리 설치

```bash
# 서버로 바이너리 복사
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@serveur:/usr/local/bin/

# 서버에서
chmod 755 /usr/local/bin/ssh-frontiere
```

## 2. 구성 설치

```bash
# 디렉토리 생성
mkdir -p /etc/ssh-frontiere

# 구성 복사
cp config.toml /etc/ssh-frontiere/config.toml

# 권한 보안 설정 (서비스 계정이 구성을 읽을 수 있어야 함)
chown root:forge-runner /etc/ssh-frontiere/config.toml
chmod 640 /etc/ssh-frontiere/config.toml

# 로그 디렉토리 생성
mkdir -p /var/log/ssh-frontiere
chown forge-runner:forge-runner /var/log/ssh-frontiere
chmod 755 /var/log/ssh-frontiere
```

## 3. 서비스 계정 생성

```bash
# ssh-frontiere를 로그인 셸로 사용하여 사용자 생성
useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

또는, 계정이 이미 있는 경우:

```bash
# 로그인 셸 변경
chsh -s /usr/local/bin/ssh-frontiere forge-runner
```

**주의**: 다른 세션에서 SSH 연결이 작동하는 것을 확인할 때까지 현재 세션을 종료하지 마세요.

## 4. SSH 키 구성 (제1계층)

`~forge-runner/.ssh/authorized_keys`를 편집합니다:

```
# CI 러너 키 (ops 수준)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci

# 모니터링 키 (read 전용)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring

# 관리자 키 (admin 수준)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-deploy
```

`command=` 옵션은 클라이언트가 보내는 명령에 관계없이 선택한 `--level`로 `ssh-frontiere`의 실행을 강제합니다. `restrict` 옵션은 포트 포워딩, 에이전트 포워딩, PTY 및 X11을 비활성화합니다.

```bash
# 권한 보안 설정
chmod 700 ~forge-runner/.ssh
chmod 600 ~forge-runner/.ssh/authorized_keys
chown -R forge-runner:forge-runner ~forge-runner/.ssh
```

## 5. sudoers 구성 (제3계층)

`/etc/sudoers.d/ssh-frontiere`를 생성합니다:

```
# SSH-Frontière: 서비스 계정에 허가된 명령
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/healthcheck.sh
```

와일드카드 `*`는 인자를 받는 스크립트에 필요합니다(예: `backup-config.sh forgejo`). 인자가 없는 스크립트(예: `healthcheck.sh`)에는 필요하지 않습니다.

문법을 검증합니다:

```bash
visudo -c -f /etc/sudoers.d/ssh-frontiere
```

## 6. 확인

```bash
# 다른 터미널에서 테스트 (현재 세션을 종료하지 마세요)

# 사용 가능한 명령이 표시되는지 확인
{ echo "help"; echo "."; } | ssh forge-runner@serveur

# 명령 테스트
{ echo "infra healthcheck"; echo "."; } | ssh forge-runner@serveur
```

## 심층 방어

세 계층이 서로 보완합니다:

| 계층 | 메커니즘 | 보호 |
|------|----------|------|
| 1 | `authorized_keys`의 `command=` + `restrict` | 수준 강제, 포워딩/PTY 차단 |
| 2 | SSH-Frontière (로그인 셸) | TOML 화이트리스트에 대해 검증 |
| 3 | sudoers의 `sudo` | 시스템 명령 제한 |

공격자가 SSH 키를 탈취하더라도, 화이트리스트에 허가된 명령만 실행할 수 있습니다. 제2계층을 우회하더라도, sudoers에 의해 권한이 제한됩니다.

## 롤백

문제가 발생하면 일반 셸로 되돌립니다:

```bash
# 콘솔(IPMI/KVM) 또는 다른 관리 계정을 통해
chsh -s /bin/bash forge-runner
```

**권장**: 로그인 셸 변경 전에 `/etc/passwd`를 백업하세요.

```bash
cp /etc/passwd /etc/passwd.bak.$(date +%Y%m%d)
```

---

**다음**: [첫 사용](@/guides/premier-usage.md) — SSH-Frontière를 통한 첫 SSH 명령.
