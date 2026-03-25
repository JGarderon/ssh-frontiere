+++
title = "CI/CD 통합"
description = "Forgejo Actions 또는 GitHub Actions에서 SSH-Frontière를 통해 배포하기"
date = 2026-03-24
weight = 5
+++

# CI/CD 통합

SSH-Frontière는 CI/CD 파이프라인과 자연스럽게 통합됩니다. 러너가 SSH를 통해 명령을 보내면, SSH-Frontière가 검증하고 실행합니다.

## Forgejo Actions

### 사전 요구사항

1. `authorized_keys`에 `--level=ops`로 구성된 러너 전용 SSH 키
2. Forgejo 저장소에 시크릿으로 저장된 개인 키 (`SSH_PRIVATE_KEY`)
3. 시크릿으로 저장된 서버 주소 (`DEPLOY_HOST`)

### 배포 워크플로

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: SSH 키 구성
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: 배포
        run: |
          {
            echo "forgejo deploy version=latest"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}

      - name: 확인
        run: |
          RESULT=$({
            echo "infra healthcheck"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
```

## GitHub Actions

### 동등한 워크플로

```yaml
name: Deploy via SSH-Frontière
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: SSH 키 구성
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: SSH-Frontière를 통해 배포
        run: |
          {
            echo "monapp deploy version=stable"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }}

      - name: 배포 확인
        run: |
          RESULT=$({
            echo "monapp status"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
          # 최종 JSON 응답 파싱 (>>> 접두사)
          STATUS=$(echo "$RESULT" | grep "^>>> " | sed 's/^>>> //' | python3 -c "import sys,json; print(json.load(sys.stdin)['status_code'])")
          if [ "$STATUS" != "0" ]; then
            echo "배포 실패 (코드 $STATUS)"
            exit 1
          fi
```

## CI/CD용 서버 구성

### 일반적인 액션

```toml
[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.deploy]
description = "Déployer une version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.forgejo.actions.rollback]
description = "Revenir à la version précédente"
level = "ops"
timeout = 120
execute = "sudo /usr/local/bin/rollback.sh {domain}"

[domains.forgejo.actions.backup-pre-deploy]
description = "Sauvegarde avant déploiement"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-pre-deploy.sh {domain}"
```

### 러너 SSH 키

```
# deploy 계정의 authorized_keys
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... forgejo-runner
```

### Sudoers

```
deploy ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/rollback.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/backup-pre-deploy.sh *
```

SSH-Frontière가 해석된 인자를 스크립트에 전달하므로(예: `deploy.sh forgejo latest`) 와일드카드 `*`가 필요합니다.

## 다단계 파이프라인

완전한 배포(백업, 배포, 확인, 알림):

```yaml
      - name: 전체 파이프라인 (백업, 배포, 확인)
        run: |
          {
            echo "+ session keepalive"
            echo "forgejo backup-pre-deploy"
            echo "."
            echo "forgejo deploy version=stable"
            echo "."
            echo "infra healthcheck"
            echo "."
            echo "."   # 빈 블록 = 세션 종료
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}
```

각 명령 뒤에 `.`(블록 끝)이 따릅니다. 앞에 명령이 없는 `.`은 세션 종료를 의미합니다. 세션 모드는 명령마다 SSH 연결을 열지 않아도 됩니다.

## 모범 사례

1. **파이프라인별 전용 키**: 러너/워크플로별 SSH 키, 필요한 최소 수준으로
2. **시크릿**: 개인 키를 코드에 저장하지 마세요 — CI 시크릿 사용
3. **배포 전 백업**: 배포 전에 항상 백업
4. **배포 후 확인**: 배포 후 healthcheck 호출
5. **롤백**: 빠른 롤백을 위한 액션 준비
6. **로그**: SSH-Frontière의 JSON 로그로 모든 배포를 추적 가능

---

**참고**: [FAQ](@/faq.md) | [대안](@/alternatives.md) | [기여](@/contribuer.md)
