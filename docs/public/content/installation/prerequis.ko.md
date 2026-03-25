+++
title = "사전 요구사항"
description = "SSH-Frontière 설치에 필요한 것"
date = 2026-03-24
weight = 1
+++

# 사전 요구사항

## 대상 서버

| 항목 | 세부사항 |
|------|----------|
| 시스템 | Linux x86_64 |
| SSH 접근 | 작동하는 `sshd` |
| 서비스 계정 | 전용 사용자 (예: `forge-runner`) |
| 비상용 관리 계정 | `/bin/bash`를 사용하는 계정 (절대 변경되지 않음) |
| 콘솔 접근 | IPMI, KVM 또는 클라우드 콘솔 — SSH 잠금 시 대비 |

**중요**: 항상 작동하는 콘솔 접근과 일반 셸을 사용하는 관리 계정을 유지하세요. SSH-Frontière 로그인 셸이 잘못 구성되면 서비스 계정의 SSH 접근을 잃을 수 있습니다.

## 빌드 머신

소스에서 SSH-Frontière를 컴파일하려면:

| 항목 | 세부사항 |
|------|----------|
| Rust | 1.70 이상 |
| musl 타겟 | `x86_64-unknown-linux-musl` (정적 바이너리용) |
| `make` | 선택사항, Makefile 단축 명령용 |

### musl 타겟 설치

```bash
rustup target add x86_64-unknown-linux-musl
```

## 대안: 사전 컴파일된 바이너리

컴파일을 원하지 않는 경우, [프로젝트 릴리스 페이지](https://github.com/nothus-forge/ssh-frontiere/releases)에서 정적 바이너리를 다운로드할 수 있습니다. 바이너리는 시스템 의존성이 없습니다.

---

**다음**: [소스에서 컴파일](@/installation/compilation.md)
