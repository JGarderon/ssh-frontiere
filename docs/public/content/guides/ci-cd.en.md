+++
title = "CI/CD integration"
description = "Deploy via SSH-Frontière from Forgejo Actions or GitHub Actions"
date = 2026-03-24
weight = 5
+++

# CI/CD integration

SSH-Frontière integrates naturally with CI/CD pipelines. The runner sends commands via SSH, SSH-Frontière validates and executes.

## Forgejo Actions

### Prerequisites

1. A dedicated SSH key for the runner, configured in `authorized_keys` with `--level=ops`
2. The private key stored as a secret in the Forgejo repository (`SSH_PRIVATE_KEY`)
3. The server address as a secret (`DEPLOY_HOST`)

### Deployment workflow

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configure SSH key
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Deploy
        run: |
          {
            echo "forgejo deploy version=latest"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}

      - name: Verify
        run: |
          RESULT=$({
            echo "infra healthcheck"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
```

## GitHub Actions

### Equivalent workflow

```yaml
name: Deploy via SSH-Frontière
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configure SSH key
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Deploy via SSH-Frontière
        run: |
          {
            echo "myapp deploy version=stable"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }}

      - name: Verify deployment
        run: |
          RESULT=$({
            echo "myapp status"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
          # Parse the final JSON response (prefixed by >>>)
          STATUS=$(echo "$RESULT" | grep "^>>> " | sed 's/^>>> //' | python3 -c "import sys,json; print(json.load(sys.stdin)['status_code'])")
          if [ "$STATUS" != "0" ]; then
            echo "Deployment failed (code $STATUS)"
            exit 1
          fi
```

## Server configuration for CI/CD

### Typical actions

```toml
[domains.forgejo]
description = "Git forge"

[domains.forgejo.actions.deploy]
description = "Deploy a version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.forgejo.actions.rollback]
description = "Roll back to the previous version"
level = "ops"
timeout = 120
execute = "sudo /usr/local/bin/rollback.sh {domain}"

[domains.forgejo.actions.backup-pre-deploy]
description = "Backup before deployment"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-pre-deploy.sh {domain}"
```

### Runner SSH key

```
# authorized_keys for the deploy account
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... forgejo-runner
```

### Sudoers

```
deploy ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/rollback.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/backup-pre-deploy.sh *
```

The wildcard `*` is necessary because SSH-Frontière passes resolved arguments to the script (e.g., `deploy.sh forgejo latest`).

## Pipeline with multiple steps

For a complete deployment (backup, deploy, verify, notify):

```yaml
      - name: Complete pipeline (backup, deploy, verify)
        run: |
          {
            echo "+ session keepalive"
            echo "forgejo backup-pre-deploy"
            echo "."
            echo "forgejo deploy version=stable"
            echo "."
            echo "infra healthcheck"
            echo "."
            echo "."   # empty block = end of session
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}
```

Each command is followed by `.` (end of block). A `.` without a preceding command signals the end of session. Session mode avoids opening an SSH connection per command.

## Best practices

1. **Dedicated key per pipeline**: one SSH key per runner/workflow, with the minimum necessary level
2. **Secrets**: never store the private key in code — use CI secrets
3. **Backup before deploy**: always back up before deploying
4. **Post-deploy verification**: call a healthcheck after deployment
5. **Rollback**: plan a rollback action to revert quickly
6. **Logs**: SSH-Frontière's JSON logs let you trace every deployment

---

**See also**: [FAQ](@/faq.md) | [Alternatives](@/alternatives.md) | [Contribute](@/contribuer.md)
