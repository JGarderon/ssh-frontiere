+++
title = "Integracion CI/CD"
description = "Desplegar via SSH-Frontière desde Forgejo Actions o GitHub Actions"
date = 2026-03-24
weight = 5
+++

# Integracion CI/CD

SSH-Frontière se integra de forma natural con los pipelines CI/CD. El runner envia comandos via SSH, SSH-Frontière valida y ejecuta.

## Forgejo Actions

### Requisitos previos

1. Una clave SSH dedicada para el runner, configurada en `authorized_keys` con `--level=ops`
2. La clave privada almacenada como secreto en el repositorio Forgejo (`SSH_PRIVATE_KEY`)
3. La direccion del servidor como secreto (`DEPLOY_HOST`)

### Workflow de despliegue

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configurar la clave SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Desplegar
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
      - name: Configurar la clave SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Desplegar via SSH-Frontière
        run: |
          {
            echo "monapp deploy version=stable"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }}

      - name: Verificar el despliegue
        run: |
          RESULT=$({
            echo "monapp status"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
          # Analizar la respuesta JSON final (prefijada por >>>)
          STATUS=$(echo "$RESULT" | grep "^>>> " | sed 's/^>>> //' | python3 -c "import sys,json; print(json.load(sys.stdin)['status_code'])")
          if [ "$STATUS" != "0" ]; then
            echo "Despliegue fallido (codigo $STATUS)"
            exit 1
          fi
```

## Configuracion del servidor para CI/CD

### Acciones tipicas

```toml
[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.deploy]
description = "Desplegar una version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.forgejo.actions.rollback]
description = "Volver a la version anterior"
level = "ops"
timeout = 120
execute = "sudo /usr/local/bin/rollback.sh {domain}"

[domains.forgejo.actions.backup-pre-deploy]
description = "Backup previo al despliegue"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-pre-deploy.sh {domain}"
```

### Clave SSH del runner

```
# authorized_keys de la cuenta deploy
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... forgejo-runner
```

### Sudoers

```
deploy ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/rollback.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/backup-pre-deploy.sh *
```

El comodin `*` es necesario porque SSH-Frontière pasa los argumentos resueltos al script (ej.: `deploy.sh forgejo latest`).

## Pipeline con varias etapas

Para un despliegue completo (backup, deploy, verify, notify):

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
            echo "."   # bloque vacio = fin de sesion
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}
```

Cada comando va seguido de `.` (fin de bloque). Un `.` sin comando previo senala el fin de sesion. El modo sesion evita abrir una conexion SSH por cada comando.

## Buenas practicas

1. **Clave dedicada por pipeline**: una clave SSH por runner/workflow, con el nivel minimo necesario
2. **Secretos**: nunca almacene la clave privada en el codigo — use los secretos del CI
3. **Backup antes del deploy**: siempre respalde antes de desplegar
4. **Verificacion post-deploy**: llame a un healthcheck despues del despliegue
5. **Rollback**: prevea una accion de rollback para volver atras rapidamente
6. **Logs**: los logs JSON de SSH-Frontière permiten rastrear cada despliegue

---

**Ver tambien**: [FAQ](@/faq.md) | [Alternativas](@/alternatives.md) | [Contribuir](@/contribuer.md)
