+++
title = "Integração CI/CD"
description = "Implementar via SSH-Frontière a partir de Forgejo Actions ou GitHub Actions"
date = 2026-03-24
weight = 5
+++

# Integração CI/CD

SSH-Frontière integra-se naturalmente com os pipelines CI/CD. O runner envia comandos via SSH, SSH-Frontière valida e executa.

## Forgejo Actions

### Pré-requisitos

1. Uma chave SSH dedicada para o runner, configurada no `authorized_keys` com `--level=ops`
2. A chave privada guardada como segredo no repositório Forgejo (`SSH_PRIVATE_KEY`)
3. O endereço do servidor como segredo (`DEPLOY_HOST`)

### Workflow de implementação

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configurar a chave SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Implementar
        run: |
          {
            echo "forgejo deploy version=latest"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}

      - name: Verificar
        run: |
          RESULT=$({
            echo "infra healthcheck"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
```

## GitHub Actions

### Workflow equivalente

```yaml
name: Deploy via SSH-Frontière
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configurar a chave SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Implementar via SSH-Frontière
        run: |
          {
            echo "monapp deploy version=stable"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }}

      - name: Verificar a implementação
        run: |
          RESULT=$({
            echo "monapp status"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
          # Analisar a resposta JSON final (prefixada por >>>)
          STATUS=$(echo "$RESULT" | grep "^>>> " | sed 's/^>>> //' | python3 -c "import sys,json; print(json.load(sys.stdin)['status_code'])")
          if [ "$STATUS" != "0" ]; then
            echo "Implementação falhou (código $STATUS)"
            exit 1
          fi
```

## Configuração do servidor para CI/CD

### Ações típicas

```toml
[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.deploy]
description = "Implementar uma versão"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.forgejo.actions.rollback]
description = "Voltar à versão anterior"
level = "ops"
timeout = 120
execute = "sudo /usr/local/bin/rollback.sh {domain}"

[domains.forgejo.actions.backup-pre-deploy]
description = "Salvaguarda antes da implementação"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-pre-deploy.sh {domain}"
```

### Chave SSH do runner

```
# authorized_keys da conta deploy
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... forgejo-runner
```

### Sudoers

```
deploy ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/rollback.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/backup-pre-deploy.sh *
```

O wildcard `*` é necessário porque SSH-Frontière passa os argumentos resolvidos ao script (ex.: `deploy.sh forgejo latest`).

## Pipeline com várias etapas

Para uma implementação completa (backup, deploy, verify, notify):

```yaml
      - name: Pipeline completo (backup, deploy, verify)
        run: |
          {
            echo "+ session keepalive"
            echo "forgejo backup-pre-deploy"
            echo "."
            echo "forgejo deploy version=stable"
            echo "."
            echo "infra healthcheck"
            echo "."
            echo "."   # bloco vazio = fim de sessão
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}
```

Cada comando é seguido de `.` (fim de bloco). Um `.` sem comando anterior assinala o fim da sessão. O modo sessão evita abrir uma conexão SSH por comando.

## Boas práticas

1. **Chave dedicada por pipeline**: uma chave SSH por runner/workflow, com o nível mínimo necessário
2. **Segredos**: nunca guardar a chave privada no código — usar os segredos do CI
3. **Backup antes de deploy**: fazer sempre uma salvaguarda antes de implementar
4. **Verificação pós-deploy**: chamar um healthcheck após a implementação
5. **Rollback**: prever uma ação de rollback para voltar atrás rapidamente
6. **Logs**: os logs JSON de SSH-Frontière permitem rastrear cada implementação

---

**Ver também**: [FAQ](@/faq.md) | [Alternativas](@/alternatives.md) | [Contribuir](@/contribuer.md)
