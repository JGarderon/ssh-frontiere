+++
title = "Implementação"
description = "Colocar SSH-Frontière em produção num servidor"
date = 2026-03-24
weight = 4
+++

# Implementação

A implementação de SSH-Frontière faz-se em 4 etapas: instalar o binário, configurar as chaves SSH, modificar o login shell e proteger com sudoers.

## 1. Instalar o binário

```bash
# Copiar o binário para o servidor
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@servidor:/usr/local/bin/

# No servidor
chmod 755 /usr/local/bin/ssh-frontiere
```

## 2. Instalar a configuração

```bash
# Criar o diretório
mkdir -p /etc/ssh-frontiere

# Copiar a configuração
cp config.toml /etc/ssh-frontiere/config.toml

# Proteger as permissões (a conta de serviço deve poder ler a config)
chown root:forge-runner /etc/ssh-frontiere/config.toml
chmod 640 /etc/ssh-frontiere/config.toml

# Criar o diretório de logs
mkdir -p /var/log/ssh-frontiere
chown forge-runner:forge-runner /var/log/ssh-frontiere
chmod 755 /var/log/ssh-frontiere
```

## 3. Criar a conta de serviço

```bash
# Criar o utilizador com ssh-frontiere como login shell
useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

Ou, se a conta já existe:

```bash
# Modificar o login shell
chsh -s /usr/local/bin/ssh-frontiere forge-runner
```

**Precaução**: não feche a sessão em curso enquanto não tiver verificado que a conexão SSH funciona a partir de outra sessão.

## 4. Configurar as chaves SSH (camada 1)

Edite `~forge-runner/.ssh/authorized_keys`:

```
# Chave runner CI (nível ops)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci

# Chave monitoring (nível read apenas)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring

# Chave admin (nível admin)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-deploy
```

A opção `command=` força a execução de `ssh-frontiere` com o `--level` escolhido, independentemente do comando enviado pelo cliente. A opção `restrict` desativa o forwarding de porta, o agent forwarding, o PTY e os X11.

```bash
# Proteger as permissões
chmod 700 ~forge-runner/.ssh
chmod 600 ~forge-runner/.ssh/authorized_keys
chown -R forge-runner:forge-runner ~forge-runner/.ssh
```

## 5. Configurar sudoers (camada 3)

Crie `/etc/sudoers.d/ssh-frontiere`:

```
# SSH-Frontière: comandos autorizados para a conta de serviço
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/healthcheck.sh
```

O wildcard `*` é necessário para os scripts que recebem argumentos (ex.: `backup-config.sh forgejo`). Os scripts sem argumentos (como `healthcheck.sh`) não precisam dele.

Valide a sintaxe:

```bash
visudo -c -f /etc/sudoers.d/ssh-frontiere
```

## 6. Verificar

```bash
# Testar a partir de outro terminal (não feche a sessão em curso)

# Verificar que os comandos disponíveis são apresentados
{ echo "help"; echo "."; } | ssh forge-runner@servidor

# Testar um comando
{ echo "infra healthcheck"; echo "."; } | ssh forge-runner@servidor
```

## Defesa em profundidade

As 3 camadas complementam-se:

| Camada | Mecanismo | Proteção |
|--------|-----------|----------|
| 1 | `command=` + `restrict` no `authorized_keys` | Força o nível, bloqueia forwarding/PTY |
| 2 | SSH-Frontière (login shell) | Valida contra a whitelist TOML |
| 3 | `sudo` nos sudoers | Restringe os comandos de sistema |

Mesmo que um atacante comprometa uma chave SSH, só pode executar os comandos autorizados na whitelist. Mesmo que contorne a camada 2, os privilégios são limitados pelos sudoers.

## Rollback

Se algo não funcionar, volte ao shell clássico:

```bash
# Via a consola (IPMI/KVM) ou outra conta admin
chsh -s /bin/bash forge-runner
```

**Conselho**: faça uma cópia de segurança de `/etc/passwd` antes de modificar o login shell.

```bash
cp /etc/passwd /etc/passwd.bak.$(date +%Y%m%d)
```

---

**Seguinte**: [Primeira utilização](@/guides/premier-usage.md) — o seu primeiro comando SSH via SSH-Frontière.
