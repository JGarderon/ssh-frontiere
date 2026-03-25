+++
title = "CI/CD-Integration"
description = "Deployment via SSH-Frontière aus Forgejo Actions oder GitHub Actions"
date = 2026-03-24
weight = 5
+++

# CI/CD-Integration

SSH-Frontière integriert sich natürlich in CI/CD-Pipelines. Der Runner sendet Befehle via SSH, SSH-Frontière validiert und führt aus.

## Forgejo Actions

### Voraussetzungen

1. Ein dedizierter SSH-Schlüssel für den Runner, konfiguriert in `authorized_keys` mit `--level=ops`
2. Der private Schlüssel als Secret im Forgejo-Repository (`SSH_PRIVATE_KEY`)
3. Die Serveradresse als Secret (`DEPLOY_HOST`)

### Deployment-Workflow

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: SSH-Schlüssel konfigurieren
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Deployen
        run: |
          {
            echo "forgejo deploy version=latest"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}

      - name: Verifizieren
        run: |
          RESULT=$({
            echo "infra healthcheck"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
```

## GitHub Actions

### Äquivalenter Workflow

```yaml
name: Deploy via SSH-Frontière
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: SSH-Schlüssel konfigurieren
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Via SSH-Frontière deployen
        run: |
          {
            echo "monapp deploy version=stable"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }}

      - name: Deployment verifizieren
        run: |
          RESULT=$({
            echo "monapp status"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
          # Finale JSON-Antwort parsen (Präfix >>>)
          STATUS=$(echo "$RESULT" | grep "^>>> " | sed 's/^>>> //' | python3 -c "import sys,json; print(json.load(sys.stdin)['status_code'])")
          if [ "$STATUS" != "0" ]; then
            echo "Deployment fehlgeschlagen (Code $STATUS)"
            exit 1
          fi
```

## Server-Konfiguration für CI/CD

### Typische Aktionen

```toml
[domains.forgejo]
description = "Git-Forge"

[domains.forgejo.actions.deploy]
description = "Eine Version deployen"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.forgejo.actions.rollback]
description = "Auf vorherige Version zurückrollen"
level = "ops"
timeout = 120
execute = "sudo /usr/local/bin/rollback.sh {domain}"

[domains.forgejo.actions.backup-pre-deploy]
description = "Sicherung vor Deployment"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-pre-deploy.sh {domain}"
```

### Runner-SSH-Schlüssel

```
# authorized_keys für das Deploy-Konto
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... forgejo-runner
```

### Sudoers

```
deploy ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/rollback.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/backup-pre-deploy.sh *
```

Der Wildcard `*` ist nötig, da SSH-Frontière aufgelöste Argumente an das Skript übergibt (z.B. `deploy.sh forgejo latest`).

## Pipeline mit mehreren Schritten

Für ein vollständiges Deployment (Backup, Deploy, Verify, Notify):

```yaml
      - name: Vollständige Pipeline (Backup, Deploy, Verify)
        run: |
          {
            echo "+ session keepalive"
            echo "forgejo backup-pre-deploy"
            echo "."
            echo "forgejo deploy version=stable"
            echo "."
            echo "infra healthcheck"
            echo "."
            echo "."   # leerer Block = Sitzungsende
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}
```

Jeder Befehl wird von `.` (Blockende) gefolgt. Ein `.` ohne vorherigen Befehl signalisiert das Sitzungsende. Der Sitzungsmodus vermeidet das Öffnen einer SSH-Verbindung pro Befehl.

## Best Practices

1. **Dedizierter Schlüssel pro Pipeline**: ein SSH-Schlüssel pro Runner/Workflow, mit der minimal nötigen Stufe
2. **Secrets**: privaten Schlüssel nie im Code speichern — CI-Secrets verwenden
3. **Backup vor Deploy**: immer vor dem Deployment sichern
4. **Post-Deploy-Verifizierung**: nach dem Deployment einen Healthcheck aufrufen
5. **Rollback**: eine Rollback-Aktion vorbereiten, um schnell zurückzurollen
6. **Logs**: SSH-Frontières JSON-Logs ermöglichen die Nachverfolgung jedes Deployments

---

**Siehe auch**: [FAQ](@/faq.md) | [Alternativen](@/alternatives.md) | [Mitwirken](@/contribuer.md)
