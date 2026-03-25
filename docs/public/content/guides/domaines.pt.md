+++
title = "Domínios e ações"
description = "Configurar domínios e ações no SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Configurar domínios e ações

Um **domínio** é um perímetro funcional (uma aplicação, um serviço, uma categoria de operações). Cada domínio contém **ações**: os comandos autorizados.

## Adicionar um domínio de implementação

```toml
[domains.minhaapp]
description = "Aplicação web principal"

[domains.minhaapp.actions.deploy]
description = "Implementar uma versão"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy-minhaapp.sh {tag}"

[domains.minhaapp.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.minhaapp.actions.status]
description = "Verificar o estado do serviço"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-minhaapp.sh"

[domains.minhaapp.actions.restart]
description = "Reiniciar o serviço"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-minhaapp.sh"
```

Utilização:

```bash
# Implementar a versão stable
{ echo "minhaapp deploy version=stable"; echo "."; } | ssh ops@servidor

# Verificar o estado
{ echo "minhaapp status"; echo "."; } | ssh monitoring@servidor

# Reiniciar
{ echo "minhaapp restart"; echo "."; } | ssh ops@servidor
```

## Adicionar um domínio de salvaguarda

```toml
[domains.backup]
description = "Salvaguardas automatizadas"

[domains.backup.actions.full]
description = "Salvaguarda completa"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"

[domains.backup.actions.config-only]
description = "Salvaguarda da configuração"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
```

## Adicionar um domínio de notificação

```toml
[domains.notify]
description = "Notificações"

[domains.notify.actions.slack]
description = "Enviar uma notificação Slack"
level = "ops"
timeout = 30
execute = "/usr/local/bin/notify-slack.sh {channel} {message}"

[domains.notify.actions.slack.args]
channel = { type = "enum", values = ["general", "ops", "alerts"], default = "ops" }
message = { free = true }
```

O argumento `message` é declarado com `free = true`: aceita qualquer valor textual.

```bash
{ echo 'notify slack channel=ops message="Implementação concluída"'; echo "."; } | ssh ops@servidor
```

## Adicionar um domínio de manutenção

```toml
[domains.infra]
description = "Infraestrutura do servidor"

[domains.infra.actions.healthcheck]
description = "Verificação de saúde dos serviços"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.disk-usage]
description = "Espaço em disco"
level = "read"
timeout = 10
execute = "/usr/bin/df -h"

[domains.infra.actions.logs]
description = "Últimos logs de sistema"
level = "ops"
timeout = 30
execute = "sudo /usr/bin/journalctl -n 100 --no-pager"
```

## Checklist após adicionar uma ação

1. Verificar a sintaxe TOML (um erro = fail-fast, código 129)
2. Criar o script de execução se necessário
3. Adicionar nos sudoers se o comando utiliza `sudo`
4. Testar com `ssh user@servidor` a partir de outro terminal
5. Verificar os logs em `/var/log/ssh-frontiere/commands.json`

## Descoberta

Os comandos `help` e `list` permitem ver as ações disponíveis:

```bash
# Lista completa com descrições (texto legível via #>)
{ echo "help"; echo "."; } | ssh user@servidor

# Detalhes de um domínio (texto legível via #>)
{ echo "help minhaapp"; echo "."; } | ssh user@servidor

# Lista curta em JSON (domínio + ação)
{ echo "list"; echo "."; } | ssh user@servidor
```

`help` devolve texto legível (prefixo `#>`). `list` devolve JSON estruturado — mais adequado ao parsing automático. Ambos mostram apenas as ações acessíveis ao nível efetivo do cliente.

---

**Seguinte**: [Tokens e níveis de segurança](@/guides/tokens.md) — controlar quem pode fazer o quê.
