+++
title = "소개"
description = "SSH-Frontière 알아보기: 무엇이고, 왜 존재하며, 어떻게 작동하는가"
date = 2026-03-24
weight = 1
+++

# SSH-Frontière 소개

## 문제점

Linux 서버에서 SSH 서비스 계정(CI 러너, AI 에이전트, 유지보수 스크립트)은 일반적으로 `/bin/bash`를 로그인 셸로 사용합니다. 이는 여러 문제를 야기합니다:

- **제어 부재**: SSH 클라이언트가 모든 명령을 실행할 수 있음
- **감사 부재**: 실행된 명령이 구조화된 방식으로 기록되지 않음
- **세분화 부재**: 상태를 읽기만 하는 스크립트와 배포 스크립트가 동일한 권한을 가짐

기존 솔루션(`authorized_keys`의 `command=`, bash 래퍼 스크립트, SSH 배스천)은 각각 한계가 있습니다: 취약하거나, 감사가 어렵거나, 필요에 비해 과도합니다.

## SSH-Frontière가 하는 일

SSH-Frontière는 **대체 로그인 셸**입니다. `sshd`와 시스템 명령 사이에 위치합니다:

```
SSH 클라이언트
    |
    v
sshd (키 인증)
    |
    v
ssh-frontiere (로그인 셸)
    |
    ├── TOML 구성에 따라 명령 검증
    ├── 접근 수준 확인 (read / ops / admin)
    ├── 허가된 명령 실행
    └── 구조화된 JSON으로 결과 반환
```

각 SSH 연결은 새로운 `ssh-frontiere` 프로세스를 생성하며, 이 프로세스는:

1. 배너와 서버 기능을 표시
2. 클라이언트 헤더를 읽음 (인증, 세션 모드)
3. 명령을 읽음 (`도메인 액션 [인자]`, 일반 텍스트)
4. TOML 화이트리스트에 대해 검증
5. 허가되면 실행, 아니면 거부
6. JSON 응답을 반환하고 종료

프로그램은 **동기식이며 일회성**입니다: 데몬 없음, 서비스 없음, 영구 상태 없음.

## SSH-Frontière가 하지 않는 것

- **SSH 배스천이 아님**: 프록시 없음, 다른 서버로의 연결 중계 없음
- **키 관리자가 아님**: SSH 키 관리는 `authorized_keys`와 `sshd`에서 처리
- **셸이 아님**: 명령 해석 없음, 파이프 없음, 리다이렉션 없음, 대화형 기능 없음
- **데몬이 아님**: 각 연결마다 실행되고 종료됨

## 구체적인 사용 사례

### CI/CD 자동화

Forgejo Actions 러너가 SSH를 통해 애플리케이션을 배포합니다:

```bash
# 러너가 SSH를 통해 명령을 전송
{
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@serveur
```

SSH-Frontière는 러너가 `admin` 수준인지, `forgejo` 도메인에 `deploy` 액션이 존재하는지, `version=stable` 인자가 허용된 값인지 확인한 후 구성된 배포 스크립트를 실행합니다.

### AI 에이전트

Claude Code 에이전트가 제한된 권한으로 서버에서 작업합니다:

```bash
# 에이전트가 사용 가능한 명령을 탐색
{ echo "list"; echo "."; } | ssh agent-ia@serveur

# 에이전트가 특정 액션을 실행
{ echo "infra healthcheck"; echo "."; } | ssh agent-ia@serveur
```

에이전트는 자신에게 구성된 `read` 수준의 액션에만 접근할 수 있습니다. `help`와 `list` 명령으로 사용 가능한 액션과 매개변수를 탐색할 수 있으며, JSON 형식으로 네이티브 파싱이 가능합니다.

### 자동화된 유지보수

cron 스크립트가 SSH를 통해 백업을 실행합니다:

```bash
# 야간 백업
{ echo "forgejo backup-config"; echo "."; } | ssh backup@serveur

# 배포 후 알림
{ echo 'notify send message="배포 완료"'; echo "."; } | ssh notify@serveur
```

### 알림

SSH-Frontière 표준 액션으로 알림(Slack, Olvid, 이메일)을 트리거합니다:

```bash
{ echo 'notify slack channel=ops message="Build OK"'; echo "."; } | ssh notify@serveur
```

## 왜 다른 것 대신 SSH-Frontière인가

### ...`authorized_keys`의 bash 스크립트 대신?

`authorized_keys`의 `command=` 옵션은 명령을 강제할 수 있지만:
- 키당 하나의 스크립트만 가능 — 세분화 없음
- 인자 검증 없음
- 접근 수준 없음
- 구조화된 로깅 없음
- bash 스크립트에 취약점이 있을 수 있음 (인젝션, 글로빙)

SSH-Frontière는 선언적 구성, RBAC, JSON 로깅, 인젝션을 원천 차단하는 문법 파서를 제공합니다.

### ...SSH 배스천(Teleport, Boundary) 대신?

SSH 배스천은 **사람**의 서버 접근을 관리하기 위해 설계되었습니다:
- 배포와 유지보수가 무거움
- 서비스 계정에는 과도함
- 다른 위협 모델 (대화형 사용자 vs 자동화 스크립트)

SSH-Frontière는 **서비스 계정**을 위해 설계된 경량 컴포넌트(약 1 Mo)입니다: 대화형 세션 없음, 프록시 없음, 명령 검증만 수행.

### ...`sudo` 단독 사용 대신?

`sudo`는 권한 상승을 제어하지만:
- SSH 클라이언트가 무엇을 *요청*할 수 있는지 제어하지 않음
- 구조화된 프로토콜 없음 (JSON 입출력)
- SSH 명령 수준의 통합 로깅 없음

SSH-Frontière와 `sudo`는 상호 보완적입니다: SSH-Frontière가 수신 명령을 검증하고(제2계층), `sudo`가 시스템 권한을 제어합니다(제3계층). 심층 방어의 제2계층과 제3계층입니다.

## 제품의 가치

SSH-Frontière는 SSH 서비스 접근에 대한 **선언적 거버넌스**를 제공합니다:

1. **모든 것이 하나의 TOML 파일에**: 도메인, 액션, 인자, 접근 수준. 스크립트에 분산된 로직 없음.

2. **즉시 배포**: 모든 구성이 하나의 TOML 파일에 집중되어 있어 새 버전 배포가 간단합니다. 각 SSH 연결은 구성을 다시 읽는 새 프로세스를 생성하므로, 변경 사항은 현재 세션이 끝나는 즉시 또는 새 클라이언트에 대해 즉시 적용됩니다.

3. **기본적으로 제로 트러스트**: 명시적으로 구성되지 않으면 아무것도 실행되지 않습니다. 셸 없음, 인젝션 불가.

4. **감사 가능**: 모든 시도(허가 또는 거부)가 타임스탬프, 명령, 인자, 수준, 결과와 함께 구조화된 JSON으로 기록됩니다.

5. **LLM 호환**: AI 에이전트가 `help`/`list`를 통해 사용 가능한 액션을 탐색하고, 구조화된 JSON 프로토콜로 상호작용할 수 있습니다 — 자유 텍스트 파싱이 필요 없습니다.

6. **유럽산 오픈소스**: EUPL-1.2 라이선스, 프랑스에서 개발, 독점 생태계에 대한 의존성 없음.

---

더 알아보기: [설치](@/installation/_index.md) | [아키텍처](@/architecture.md) | [보안](@/securite.md) | [대안](@/alternatives.md)
