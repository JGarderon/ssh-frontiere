+++
title = "첫 사용"
description = "SSH-Frontière 설치, 첫 도메인 구성 및 테스트"
date = 2026-03-24
weight = 1
+++

# 첫 사용

이 가이드는 설치부터 SSH-Frontière를 통한 첫 SSH 명령까지 안내합니다.

## 1. 최소 구성 준비

최소 `config.toml` 파일을 만듭니다:

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 60

[domains.test]
description = "Domaine de test"

[domains.test.actions.hello]
description = "Commande de test qui affiche un message"
level = "read"
timeout = 10
execute = "/usr/bin/echo hello from ssh-frontiere"
```

이 구성은 `read` 수준에서 접근 가능한 `hello` 액션을 가진 단일 도메인 `test`를 정의합니다.

## 2. 설치 및 구성

먼저 `ssh-frontiere` 바이너리가 필요합니다. [컴파일 가이드](@/installation/compilation.md)를 참조하거나 [릴리스 페이지](https://github.com/nothus-forge/ssh-frontiere/releases)에서 사전 컴파일된 바이너리를 다운로드하세요.

```bash
# 바이너리 복사
sudo cp ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere

# 구성 설치
sudo mkdir -p /etc/ssh-frontiere
sudo cp config.toml /etc/ssh-frontiere/config.toml
sudo chmod 640 /etc/ssh-frontiere/config.toml

# 로그 디렉토리 생성
sudo mkdir -p /var/log/ssh-frontiere

# 서비스 계정 생성
sudo useradd -m -s /usr/local/bin/ssh-frontiere test-user

# 계정에 로그 쓰기 권한 부여
sudo chown test-user:test-user /var/log/ssh-frontiere
```

## 3. SSH 키 구성

클라이언트 머신에서:

```bash
# 키 생성
ssh-keygen -t ed25519 -C "test-key" -f ~/.ssh/test-frontiere
```

서버에서, 공개 키를 `~test-user/.ssh/authorized_keys`에 추가합니다:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... test-key
```

```bash
# 권한 보안 설정
sudo chmod 700 ~test-user/.ssh
sudo chmod 600 ~test-user/.ssh/authorized_keys
sudo chown -R test-user:test-user ~test-user/.ssh
```

## 4. 첫 호출

```bash
# 사용 가능한 명령 탐색
{ echo "help"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

예상 응답 (서버가 먼저 배너를 보내고 이어서 응답):

```
#> ssh-frontiere 0.1.0
+> capabilities session, help, body
#> type "help" for available commands
#> ...
>>> {"command":"help","status_code":0,"status_message":"ok","stdout":null,"stderr":null}
```

`#>` 줄에는 사람이 읽을 수 있는 도움말 텍스트가 포함됩니다. `help` 명령은 `read` 수준에서 접근 가능한 도메인과 액션 목록을 표시합니다.

## 5. 명령 실행

```bash
{ echo "test hello"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

예상 응답:

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

프로그램 출력(`hello from ssh-frontiere`)은 `>>`를 통해 스트리밍으로 전송되고, 이어서 `>>>`를 통해 최종 JSON 응답이 전송됩니다. 출력이 스트리밍으로 전송되었기 때문에 JSON의 `stdout`과 `stderr` 필드는 `null`입니다.

## 6. 흐름 이해

다음과 같은 과정이 있었습니다:

1. SSH 클라이언트가 `test-frontiere` 키로 연결
2. `sshd`가 키를 인증하고 `authorized_keys`를 읽음
3. `command=` 옵션이 `ssh-frontiere --level=read` 실행을 강제
4. SSH-Frontière가 배너(`#>`, `+>`)를 표시하고 헤더를 대기
5. 클라이언트가 명령 `test hello` (접두사 없는 일반 텍스트)를 보내고 `.` (블록 끝)
6. SSH-Frontière가 검증: 도메인 `test`, 액션 `hello`, 수준 `read` <= 요구되는 `read`
7. SSH-Frontière가 `/usr/bin/echo hello from ssh-frontiere`를 실행
8. 출력이 스트리밍으로 전송(`>>`)된 후 최종 JSON 응답(`>>>`)

## 7. 거부 테스트

존재하지 않는 명령을 시도합니다:

```bash
{ echo "test inexistant"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

응답:

```
>>> {"command":"test inexistant","status_code":128,"status_message":"rejected: unknown action 'inexistant' in domain 'test'","stdout":null,"stderr":null}
```

명령이 실행되지 않았으므로 `stdout`과 `stderr`는 `null`입니다.

## 다음 단계

SSH-Frontière가 동작하게 되었으니, [자신만의 도메인과 액션을 구성](@/guides/domaines.md)할 수 있습니다.
