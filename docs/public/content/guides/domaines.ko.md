+++
title = "도메인과 액션"
description = "SSH-Frontière에서 도메인과 액션 구성하기"
date = 2026-03-24
weight = 2
+++

# 도메인과 액션 구성

**도메인**은 기능적 영역(애플리케이션, 서비스, 운영 카테고리)입니다. 각 도메인은 **액션**(허가된 명령)을 포함합니다.

## 배포 도메인 추가

```toml
[domains.monapp]
description = "Application web principale"

[domains.monapp.actions.deploy]
description = "Déployer une version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy-monapp.sh {tag}"

[domains.monapp.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.monapp.actions.status]
description = "Vérifier l'état du service"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-monapp.sh"

[domains.monapp.actions.restart]
description = "Redémarrer le service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-monapp.sh"
```

사용법:

```bash
# 안정 버전 배포
{ echo "monapp deploy version=stable"; echo "."; } | ssh ops@serveur

# 상태 확인
{ echo "monapp status"; echo "."; } | ssh monitoring@serveur

# 재시작
{ echo "monapp restart"; echo "."; } | ssh ops@serveur
```

## 백업 도메인 추가

```toml
[domains.backup]
description = "Sauvegardes automatisées"

[domains.backup.actions.full]
description = "Sauvegarde complète"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"

[domains.backup.actions.config-only]
description = "Sauvegarde de la configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
```

## 알림 도메인 추가

```toml
[domains.notify]
description = "Notifications"

[domains.notify.actions.slack]
description = "Envoyer une notification Slack"
level = "ops"
timeout = 30
execute = "/usr/local/bin/notify-slack.sh {channel} {message}"

[domains.notify.actions.slack.args]
channel = { type = "enum", values = ["general", "ops", "alerts"], default = "ops" }
message = { free = true }
```

`message` 인자는 `free = true`로 선언되어 모든 텍스트 값을 허용합니다.

```bash
{ echo 'notify slack channel=ops message="배포 완료"'; echo "."; } | ssh ops@serveur
```

## 유지보수 도메인 추가

```toml
[domains.infra]
description = "Infrastructure serveur"

[domains.infra.actions.healthcheck]
description = "Vérification de santé des services"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.disk-usage]
description = "Espace disque"
level = "read"
timeout = 10
execute = "/usr/bin/df -h"

[domains.infra.actions.logs]
description = "Derniers logs système"
level = "ops"
timeout = 30
execute = "sudo /usr/bin/journalctl -n 100 --no-pager"
```

## 액션 추가 후 체크리스트

1. TOML 문법 확인 (오류 = fail-fast, 코드 129)
2. 필요한 경우 실행 스크립트 생성
3. 명령이 `sudo`를 사용하면 sudoers에 추가
4. 다른 터미널에서 `ssh user@serveur`로 테스트
5. `/var/log/ssh-frontiere/commands.json`에서 로그 확인

## 탐색

`help`와 `list` 명령으로 사용 가능한 액션을 확인할 수 있습니다:

```bash
# 설명이 포함된 전체 목록 (#>를 통한 사람이 읽을 수 있는 텍스트)
{ echo "help"; echo "."; } | ssh user@serveur

# 도메인 세부정보 (#>를 통한 사람이 읽을 수 있는 텍스트)
{ echo "help monapp"; echo "."; } | ssh user@serveur

# JSON 형태의 짧은 목록 (도메인 + 액션)
{ echo "list"; echo "."; } | ssh user@serveur
```

`help`는 사람이 읽을 수 있는 텍스트(접두사 `#>`)를 반환합니다. `list`는 구조화된 JSON을 반환하여 자동 파싱에 더 적합합니다. 둘 다 클라이언트의 유효 수준에서 접근 가능한 액션만 표시합니다.

---

**다음**: [토큰과 보안 수준](@/guides/tokens.md) — 누가 무엇을 할 수 있는지 제어.
