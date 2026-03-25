+++
title = "Intégration CI/CD"
description = "Déployer via SSH-Frontière depuis Forgejo Actions ou GitHub Actions"
date = 2026-03-24
weight = 5
+++

# Intégration CI/CD

SSH-Frontière s'intègre naturellement avec les pipelines CI/CD. Le runner envoie des commandes via SSH, SSH-Frontière valide et exécute.

## Forgejo Actions

### Prérequis

1. Une clé SSH dédiée pour le runner, configurée dans `authorized_keys` avec `--level=ops`
2. La clé privée stockée comme secret dans le dépôt Forgejo (`SSH_PRIVATE_KEY`)
3. L'adresse du serveur en secret (`DEPLOY_HOST`)

### Workflow de déploiement

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configurer la clé SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Déployer
        run: |
          {
            echo "forgejo deploy version=latest"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}

      - name: Vérifier
        run: |
          RESULT=$({
            echo "infra healthcheck"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
```

## GitHub Actions

### Workflow équivalent

```yaml
name: Deploy via SSH-Frontière
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configurer la clé SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Déployer via SSH-Frontière
        run: |
          {
            echo "monapp deploy version=stable"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }}

      - name: Vérifier le déploiement
        run: |
          RESULT=$({
            echo "monapp status"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
          # Parser la réponse JSON finale (préfixée par >>>)
          STATUS=$(echo "$RESULT" | grep "^>>> " | sed 's/^>>> //' | python3 -c "import sys,json; print(json.load(sys.stdin)['status_code'])")
          if [ "$STATUS" != "0" ]; then
            echo "Déploiement échoué (code $STATUS)"
            exit 1
          fi
```

## Configuration serveur pour CI/CD

### Actions typiques

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

### Clé SSH du runner

```
# authorized_keys du compte deploy
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... forgejo-runner
```

### Sudoers

```
deploy ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/rollback.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/backup-pre-deploy.sh *
```

Le wildcard `*` est nécessaire car SSH-Frontière passe les arguments résolus au script (ex: `deploy.sh forgejo latest`).

## Pipeline avec plusieurs étapes

Pour un déploiement complet (backup, deploy, verify, notify) :

```yaml
      - name: Pipeline complet (backup, deploy, verify)
        run: |
          {
            echo "+ session keepalive"
            echo "forgejo backup-pre-deploy"
            echo "."
            echo "forgejo deploy version=stable"
            echo "."
            echo "infra healthcheck"
            echo "."
            echo "."   # bloc vide = fin de session
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}
```

Chaque commande est suivie de `.` (fin de bloc). Un `.` sans commande précédente signale la fin de session. Le mode session évite d'ouvrir une connexion SSH par commande.

## Bonnes pratiques

1. **Clé dédiée par pipeline** : une clé SSH par runner/workflow, avec le niveau minimum nécessaire
2. **Secrets** : ne jamais stocker la clé privée dans le code — utiliser les secrets du CI
3. **Backup avant deploy** : toujours sauvegarder avant de déployer
4. **Vérification post-deploy** : appeler un healthcheck après le déploiement
5. **Rollback** : prévoir une action de rollback pour revenir en arrière rapidement
6. **Logs** : les logs JSON de SSH-Frontière permettent de tracer chaque déploiement

---

**Voir aussi** : [FAQ](@/faq.md) | [Alternatives](@/alternatives.md) | [Contribuer](@/contribuer.md)
