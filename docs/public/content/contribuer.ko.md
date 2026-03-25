+++
title = "기여"
description = "SSH-Frontière에 기여하는 방법: 프로세스, 요구사항, 규칙"
date = 2026-03-24
weight = 6
+++

# SSH-Frontière에 기여하기

AI가 보조하거나 생성한 기여를 포함하여 모든 기여를 환영합니다. SSH-Frontière 자체도 Claude Code 에이전트로 개발되었습니다.

## 시작하기 전에

제안하는 변경에 대해 논의하기 위해 **이슈**를 열어주세요. 불필요한 작업을 방지하고 접근 방식을 검증할 수 있습니다.

- **버그**: 관찰된 동작과 예상 동작, 버전, OS를 설명해 주세요
- **기능**: 사용 사례와 제안하는 접근 방식을 설명해 주세요
- **아키텍처 변경**: ADR이 필요합니다 (`docs/decisions/` 참조)

## 프로세스

```
1. 이슈       → 변경 논의
2. Fork        → git checkout -b feature/my-contribution
3. TDD         → RED (실패하는 테스트) → GREEN (최소 코드) → 리팩터링
4. 검증        → make lint && make test && make audit
5. Pull request → 설명, 이슈 참조, 녹색 CI
```

## 품질 요구사항

SSH-Frontière는 보안 컴포넌트입니다. 요구사항이 엄격합니다:

| 규칙 | 세부사항 |
|------|----------|
| 테스트 커버리지 | 추가된 코드에 대해 최소 90% |
| `unwrap()` 금지 | `// INVARIANT:` 주석이 있는 `expect()` 또는 `?` / `map_err()` 사용 |
| `unsafe` 금지 | `#[deny(unsafe_code)]`로 금지됨 |
| 최대 800줄 | 소스 파일당 |
| 최대 60줄 | 함수당 |
| 포맷팅 | `cargo fmt` 필수 |
| 린트 | `cargo clippy -- -D warnings` (pedantic) |

### 의존성

**필수적이지 않은 의존성은 제로입니다.** 새로운 의존성을 제안하기 전에:

1. Rust 표준 라이브러리가 요구를 충족하지 않는지 확인
2. 의존성 매트릭스로 평가 (최소 점수 3.5/5)
3. `docs/searches/`에 평가를 문서화

현재 허용된 의존성: `serde`, `serde_json`, `toml`.

## 커밋 규칙

메시지는 **영어**로, `type(scope): description` 형식:

- `feat(protocol): add TLS support`
- `fix(dispatch): handle empty arguments`
- `test(integration): add session timeout scenarios`
- `docs(references): update configuration guide`

유형: `feat`, `fix`, `refactor`, `test`, `docs`.

## AI 기여

AI가 생성한 기여는 사람의 기여와 동일한 조건으로 수용됩니다:

- 사람 기여자가 코드 품질에 대한 **책임**을 유지
- 동일한 테스트 및 린트 요구사항
- PR에서 AI 코드 사용 여부를 명시 (투명성)

## 보안

### 취약점 신고

**공개 이슈를 통해 취약점을 신고하지 마세요.** 책임 있는 공개를 위해 메인테이너에게 직접 연락하세요.

### 강화된 리뷰

다음 파일에 영향을 주는 PR은 강화된 보안 리뷰를 받습니다:

- `protocol.rs`, `crypto.rs` — 인증
- `dispatch.rs`, `chain_parser.rs`, `chain_exec.rs` — 명령 파싱 및 실행
- `config.rs` — 구성 관리

## 좋은 첫 기여

- 문서 개선
- 경계 사례에 대한 테스트 추가
- clippy 경고 수정
- 오류 메시지 개선

## 라이선스

SSH-Frontière는 [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)로 배포됩니다. Pull request를 제출함으로써, 귀하의 기여가 이 라이선스 조건에 따라 배포되는 것에 동의합니다.

자세한 내용은 저장소의 [CONTRIBUTING.md](https://github.com/nothus-forge/ssh-frontiere/blob/main/CONTRIBUTING.md) 파일을 참조하세요.
