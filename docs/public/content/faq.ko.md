+++
title = "FAQ"
description = "SSH-Frontière에 대한 자주 묻는 질문"
date = 2026-03-24
weight = 5
+++

# 자주 묻는 질문

## SSH-Frontière가 정확히 무엇인가요?

Rust로 작성된 **대체 로그인 셸**입니다. 서비스 계정의 `/etc/passwd`에서 `/bin/bash` 대신 설치됩니다. 각 SSH 연결이 SSH-Frontière를 거치며, TOML 구성 파일에 따라 명령을 검증한 후 실행합니다.

## SSH 배스천인가요?

아닙니다. SSH 배스천(Teleport, Boundary)은 다른 서버로 연결을 중계하는 **프록시**입니다. SSH-Frontière는 중계를 하지 않으며, **설치된 서버에서** 실행되는 것을 제어합니다.

배스천은 사람의 서버 팜 접근을 관리합니다. SSH-Frontière는 **서비스 계정**(CI 러너, AI 에이전트, 스크립트)의 서버 내 특정 액션 접근을 관리합니다.

## `sudo`를 대체하나요?

아닙니다, 상호 보완적입니다. SSH-Frontière는 SSH 클라이언트가 **요청할 수 있는 것**을 제어합니다(제2계층). `sudo`는 **실행에 필요한** 시스템 권한을 제어합니다(제3계층). 둘을 결합하면 심층 방어가 됩니다.

## TOML 파일 없이 사용할 수 있나요?

아닙니다. 구성 파일은 필수입니다. 이것은 의도된 것입니다: 모든 것이 명시적이고, 선언적이며, 감사 가능합니다. 허용 모드도 없고, 셸로의 폴백도 없습니다.

## 구성이 잘못되면 어떻게 되나요?

SSH-Frontière는 시작 시 구성을 완전히 검증합니다(fail-fast). 구성이 잘못되면 프로그램이 종료 코드 129와 함께 명시적인 오류 메시지를 로그에 남기고 중단됩니다. 어떤 명령도 실행되지 않습니다. SSH 클라이언트는 오류 세부 정보를 **절대 보지 못합니다** — 서비스가 사용 불가능하다는 것만 알 수 있습니다. 진단 정보는 서버 측에 남습니다.

위험 없이 구성을 테스트할 수 있습니다:

```bash
ssh-frontiere --check-config --config /etc/ssh-frontiere/config.toml
```

## 문제를 어떻게 진단하나요?

여러 도구가 제공됩니다:

1. **구성 검증**: `ssh-frontiere --check-config`로 문법과 일관성 검증
2. **`help` 명령**: 클라이언트의 유효 수준에서 접근 가능한 액션 표시
3. **`list` 명령**: 짧은 버전 (도메인 + 액션)
4. **JSON 로그**: 실행되거나 거부된 모든 명령이 타임스탬프, 명령, 인자, 수준, 결과와 함께 기록됨
5. **종료 코드**: 0 = 성공, 128 = 거부, 129 = 구성 오류, 130 = 타임아웃, 131 = 수준 부족, 132 = 프로토콜 오류, 133 = body stdin 조기 종료

## AI 에이전트가 사용할 수 있나요?

네, 이것은 일급 사용 사례입니다. `help`와 `list` 명령은 에이전트가 직접 파싱할 수 있는 구조화된 JSON을 반환합니다. 헤더 프로토콜(접두사 `+`, `#`, `$`, `>`)은 사람의 가독성을 해치지 않으면서 기계가 읽을 수 있도록 설계되었습니다.

자세한 구성은 [AI 에이전트 가이드](@/guides/agents-ia.md)를 참조하세요.

## 소스 코드의 의존성은 무엇인가요?

직접 의존성 3개:

| 크레이트 | 용도 |
|----------|------|
| `serde` + `serde_json` | JSON 직렬화 (로그, 응답) |
| `toml` | 구성 로드 |

async 런타임 없음, Tokio 없음, 웹 프레임워크 없음. 정적 바이너리 크기는 약 1 Mo.

## 왜 Go/Python이 아니라 Rust인가요?

1. **메모리 안전성**: 버퍼 오버플로 없음, use-after-free 없음 — 보안 컴포넌트에 결정적
2. **정적 바이너리**: musl로 컴파일, 시스템 의존성 없음
3. **성능**: 밀리초 단위 시작, 런타임 없음
4. **`unsafe` 없음**: Cargo 린트로 금지 (`unsafe_code = "deny"`)

## 왜 YAML이나 JSON이 아니라 TOML인가요?

- **TOML**: 읽기 쉬움, 타입 지정, 주석, Rust 표준, 유의미한 들여쓰기 없음
- **YAML**: 유의미한 들여쓰기가 오류의 원인, 위험한 암묵적 타입(`on`/`off` → 불리언)
- **JSON**: 주석 없음, 장황함, 사람의 구성을 위해 설계되지 않음

이 선택은 ADR 0001에 문서화되어 있습니다.

## 토큰 인증은 어떻게 작동하나요?

두 가지 모드:

1. **단순 모드** (`challenge_nonce = false`): 클라이언트가 `SHA-256(secret)`을 계산하여 proof로 전송
2. **논스 모드** (`challenge_nonce = true`): 서버가 논스를 전송, 클라이언트가 `SHA-256(XOR_encrypt(secret || nonce, secret))`을 계산

논스 모드는 재사용 공격을 방지합니다: 논스 덕분에 각 proof가 고유합니다.

## 여러 SSH 키를 사용할 수 있나요?

네. `authorized_keys`의 각 키에 고유한 `--level`이 있습니다. 다른 수준의 여러 키가 공존할 수 있습니다:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin
```

## 응답 형식은 어떤가요?

표준 출력과 오류 출력은 스트리밍으로 전송(접두사 `>>`와 `>>!`)된 후, 한 줄의 최종 JSON 응답(접두사 `>>>`)이 이어집니다:

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- 최종 JSON에서 `stdout`/`stderr` = `null`: 출력이 스트리밍으로 전송되었음
- `status_code` = 0: 성공 (자식 프로세스의 종료 코드 패스스루)

## SSH-Frontière를 어떻게 업데이트하나요?

1. 새 버전 컴파일 (`make release`)
2. 서버에 바이너리 복사 (`scp`)
3. 확인 (`ssh user@serveur` + `help`)

데이터 마이그레이션 없음, 데이터베이스 스키마 없음. TOML 파일은 git으로 버전 관리 가능합니다.

## 어떻게 기여하나요?

[기여 가이드](@/contribuer.md)를 참조하세요. 요약: 이슈 열기, fork, TDD, pull request, 녹색 CI. AI 생성 기여도 수용됩니다.

## 소스 코드는 어디에서 찾을 수 있나요?

소스 코드는 [GitHub 저장소](https://github.com/nothus-forge/ssh-frontiere)에서 확인할 수 있습니다. 라이선스 [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
