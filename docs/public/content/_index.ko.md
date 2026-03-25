+++
title = "SSH-Frontière"
description = "Rust로 작성된 제한된 SSH 로그인 셸 — 수신 연결의 선언적 제어"
sort_by = "weight"

[extra]
framed = true
+++

# SSH-Frontière

**Rust로 작성된 제한된 SSH 로그인 셸** — 모든 수신 SSH 연결을 위한 단일하고 안전한 진입점.

SSH-Frontière는 Unix 계정의 기본 셸(`/bin/bash`)을 TOML 선언적 구성에 따라 **각 명령을 검증**한 후 실행하는 프로그램으로 대체합니다.

[![GitHub](https://img.shields.io/badge/GitHub-오픈소스_저장소-181717?logo=github&logoColor=white&style=for-the-badge)](https://github.com/JGarderon/ssh-frontiere)

---

## 왜 SSH-Frontière인가?

**기본적으로 안전** — 명시적으로 허용되지 않은 명령은 실행되지 않습니다. 기본 거부(deny by default), 셸 없음, 인젝션 불가.

**간편한 배포** — 약 1 Mo의 정적 바이너리 하나, TOML 파일 하나, `/etc/passwd`에 한 줄. 데몬 없음, 관리할 서비스 없음.

**유연함** — 세 가지 접근 수준(read, ops, admin), 가시성 태그, 구조화된 헤더 프로토콜. AI 에이전트, CI/CD 러너, 유지보수 스크립트와 호환.

**감사 가능** — 실행되거나 거부된 모든 명령이 구조화된 JSON으로 기록됩니다. 399개의 cargo 테스트 + 72개의 E2E SSH 시나리오.

---

## 사용 사례

- **CI/CD 러너** (Forgejo Actions, GitHub Actions): SSH를 통한 배포, 백업, 상태 점검
- **AI 에이전트** (Claude Code 등): 신뢰 수준에 따른 서버 리소스 접근 제어
- **자동화된 유지보수**: 백업, 모니터링, 알림 스크립트

---

## 한눈에 보기

| | |
|---|---|
| **언어** | Rust (musl 정적 바이너리, 약 1 Mo) |
| **라이선스** | [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) — 유럽 연합 공공 라이선스 |
| **테스트** | 399 cargo + 72 E2E SSH + 9 fuzz harness |
| **의존성** | 직접 의존성 3개 (`serde`, `serde_json`, `toml`) |
| **구성** | TOML 선언적 구성 |
| **프로토콜** | stdin/stdout 텍스트 헤더, JSON 응답 |

---

## 시작하기

- [SSH-Frontière 알아보기](@/presentation.md) — 무엇이고, 무엇을 하며, 왜 존재하는가
- [설치](@/installation/_index.md) — 컴파일, 구성, 배포
- [가이드](@/guides/_index.md) — 단계별 튜토리얼
- [보안](@/securite.md) — 보안 모델과 보장 사항
- [아키텍처](@/architecture.md) — 기술 설계
- [대안](@/alternatives.md) — 다른 솔루션과의 비교
- [FAQ](@/faq.md) — 자주 묻는 질문
- [기여](@/contribuer.md) — 프로젝트에 참여하기
