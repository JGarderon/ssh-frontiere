+++
title = "컴파일"
description = "소스에서 SSH-Frontière 컴파일하기"
date = 2026-03-24
weight = 2
+++

# 소스에서 컴파일

## 릴리스 컴파일

```bash
# make 사용 (권장)
make release

# 또는 cargo 직접 사용
cargo build --release --target x86_64-unknown-linux-musl
```

결과 바이너리는 다음 위치에 생성됩니다:

```
target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

이것은 약 1 Mo의 **정적 바이너리**로, 시스템 의존성이 없습니다. 모든 Linux x86_64 서버에 직접 복사할 수 있습니다.

## 확인

```bash
# 바이너리 유형 확인
file target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ELF 64-bit LSB executable, x86-64, statically linked

# 크기 확인
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ~1-2 Mo
```

## 디버그 컴파일

개발용:

```bash
make build
# 또는
cargo build
```

## 테스트

배포 전에 테스트가 통과하는지 확인하세요:

```bash
# 단위 테스트 및 통합 테스트
make test

# 린트 (포맷팅 + clippy)
make lint

# 의존성 감사
make audit
```

## 보조 바이너리: proof

인증 proof를 계산하기 위한 보조 바이너리가 포함되어 있습니다:

```bash
cargo build --release --target x86_64-unknown-linux-musl --bin proof
```

이 바이너리는 클라이언트 측에서 SHA-256 계산을 구현하지 않고도 챌린지-응답 인증을 테스트하는 데 유용합니다.

---

**다음**: [구성](@/installation/configuration.md) — `config.toml` 파일 준비.
