+++
title = "AI 에이전트"
description = "SSH-Frontière를 AI 에이전트(Claude Code 등)와 함께 사용하기"
date = 2026-03-24
weight = 4
+++

# AI 에이전트와 SSH-Frontière 사용

SSH-Frontière는 처음부터 AI 에이전트(LLM)와의 호환성을 고려하여 설계되었습니다. 구조화된 프로토콜, 자동 탐색, JSON 응답은 서버에서 작업해야 하는 에이전트를 위한 이상적인 진입점을 만듭니다.

## 왜 AI 에이전트에 SSH-Frontière인가?

AI 에이전트(Claude Code, Cursor, GPT 등)는 SSH를 통해 서버에서 명령을 실행할 수 있습니다. 문제는: 제어가 없으면 에이전트가 무엇이든 실행할 수 있다는 것입니다.

SSH-Frontière는 이 문제를 해결합니다:

- **액션 제한**: 에이전트는 구성된 명령만 실행 가능
- **접근 수준**: `read` 수준의 에이전트는 조회만 가능, 수정 불가
- **탐색**: 에이전트가 `help`를 요청하여 사용 가능한 액션 확인 가능
- **구조화된 JSON**: 응답을 에이전트가 직접 파싱 가능

## AI 에이전트를 위한 구성

### 1. 전용 SSH 키

에이전트용 SSH 키를 생성합니다:

```bash
ssh-keygen -t ed25519 -C "agent-claude" -f ~/.ssh/agent-claude
```

### 2. 제한된 신뢰 수준

`authorized_keys`에서 최소 수준을 부여합니다:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent-claude
```

`read`로 시작하고 필요한 경우 토큰으로 상승합니다.

### 3. 전용 도메인

에이전트를 위한 특정 액션을 구성합니다:

```toml
[domains.agent]
description = "Actions pour agents IA"

[domains.agent.actions.status]
description = "État des services"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-all.sh"

[domains.agent.actions.logs]
description = "Derniers logs applicatifs"
level = "read"
timeout = 30
execute = "/usr/local/bin/recent-logs.sh {service}"

[domains.agent.actions.logs.args]
service = { type = "enum", values = ["web", "api", "worker", "database"] }

[domains.agent.actions.restart]
description = "Redémarrer un service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-service.sh {service}"
tags = ["agent-ops"]

[domains.agent.actions.restart.args]
service = { type = "enum", values = ["web", "api", "worker"] }
```

### 4. 상승용 토큰 (선택사항)

에이전트가 `ops` 액션에 접근해야 하는 경우:

```toml
[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "ops"
tags = ["agent-ops"]
```

## Claude Code (AutoClaude) 예제

AutoClaude 컨테이너의 Claude Code 에이전트가 SSH-Frontière를 사용하여 호스트 서버에서 작업할 수 있습니다:

```bash
# 에이전트가 사용 가능한 명령을 탐색 (list로 JSON 획득)
{ echo "list"; echo "."; } | ssh -i /keys/agent-claude agent@serveur

# 에이전트가 서비스 상태 확인
{ echo "agent status"; echo "."; } | ssh -i /keys/agent-claude agent@serveur

# 에이전트가 서비스의 로그를 읽음
{ echo "agent logs service=api"; echo "."; } | ssh -i /keys/agent-claude agent@serveur
```

출력은 스트리밍(`>>`)으로 전송되고, 이어서 최종 JSON 응답(`>>>`)이 전송됩니다:

```
>> web: running
>> api: running
>> worker: stopped
>>> {"command":"agent status","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

에이전트는 `>>` 줄(스트리밍 표준 출력)을 분석하고, `worker`가 중지되었음을 감지하여 그에 따라 조치를 결정할 수 있습니다. `>>>` 응답은 반환 코드를 확인합니다.

## 세션 모드

명령마다 SSH 연결을 열지 않기 위해, 에이전트는 세션 모드를 사용할 수 있습니다:

```bash
{
  echo "+ auth token=agent-claude proof=..."
  echo "+ session keepalive"
  echo "agent status"
  echo "."
  echo "agent logs service=worker"
  echo "."
  echo "."   # 빈 블록 = 세션 종료
} | ssh -i /keys/agent-claude agent@serveur
```

각 명령 뒤에 `.`(블록 끝)이 따릅니다. 앞에 명령이 없는 `.`은 세션 종료를 의미합니다. 세션 모드는 단일 SSH 연결에서 여러 명령을 보낼 수 있으며, 구성 가능한 전체 타임아웃(`timeout_session`)이 있습니다.

## 모범 사례

1. **최소 권한 원칙**: `read`로 시작하고, 필요한 경우에만 토큰으로 상승
2. **원자적 액션**: 각 액션은 하나의 작업만 수행. 에이전트가 액션을 조합
3. **명시적 이름**: 도메인과 액션 이름은 `help`에서 보이므로 이해하기 쉽게 작성
4. **가시성 태그**: 전용 태그로 에이전트의 액션을 격리
5. **출력 제한**: 에이전트가 과도한 데이터를 받지 않도록 `max_stdout_chars` 구성
6. **로그**: 비정상적인 사용을 감지하기 위해 로그 모니터링

---

**다음**: [CI/CD 통합](@/guides/ci-cd.md) — SSH-Frontière를 통한 배포 자동화.
