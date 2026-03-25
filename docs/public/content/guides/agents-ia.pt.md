+++
title = "Agentes IA"
description = "Usar SSH-Frontière com agentes IA (Claude Code, etc.)"
date = 2026-03-24
weight = 4
+++

# Usar SSH-Frontière com agentes IA

SSH-Frontière foi concebido desde a origem para ser compatível com agentes IA (LLM). O protocolo estruturado, a descoberta automática e as respostas JSON fazem dele um ponto de entrada ideal para agentes que precisam de atuar num servidor.

## Porquê SSH-Frontière para agentes IA?

Os agentes IA (Claude Code, Cursor, GPT, etc.) podem executar comandos num servidor via SSH. O problema: sem controlo, um agente pode executar qualquer coisa.

SSH-Frontière resolve este problema:

- **Limitar as ações**: o agente só pode executar os comandos configurados
- **Níveis de acesso**: um agente em `read` só pode consultar, não modificar
- **Descoberta**: o agente pode pedir `help` para conhecer as ações disponíveis
- **JSON estruturado**: as respostas são diretamente analisáveis pelo agente

## Configuração para um agente IA

### 1. Chave SSH dedicada

Gere uma chave SSH para o agente:

```bash
ssh-keygen -t ed25519 -C "agent-claude" -f ~/.ssh/agent-claude
```

### 2. Nível de confiança restrito

No `authorized_keys`, atribua um nível mínimo:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent-claude
```

Comece por `read` e eleve se necessário via um token.

### 3. Domínios dedicados

Configure ações específicas para o agente:

```toml
[domains.agent]
description = "Ações para agentes IA"

[domains.agent.actions.status]
description = "Estado dos serviços"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-all.sh"

[domains.agent.actions.logs]
description = "Últimos logs aplicativos"
level = "read"
timeout = 30
execute = "/usr/local/bin/recent-logs.sh {service}"

[domains.agent.actions.logs.args]
service = { type = "enum", values = ["web", "api", "worker", "database"] }

[domains.agent.actions.restart]
description = "Reiniciar um serviço"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-service.sh {service}"
tags = ["agent-ops"]

[domains.agent.actions.restart.args]
service = { type = "enum", values = ["web", "api", "worker"] }
```

### 4. Token para elevação (opcional)

Se o agente precisar de aceder a ações `ops`:

```toml
[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "ops"
tags = ["agent-ops"]
```

## Exemplo com Claude Code (AutoClaude)

Um agente Claude Code num contentor AutoClaude pode usar SSH-Frontière para atuar no servidor anfitrião:

```bash
# O agente descobre os comandos disponíveis (JSON via list)
{ echo "list"; echo "."; } | ssh -i /keys/agent-claude agent@servidor

# O agente verifica o estado dos serviços
{ echo "agent status"; echo "."; } | ssh -i /keys/agent-claude agent@servidor

# O agente lê os logs de um serviço
{ echo "agent logs service=api"; echo "."; } | ssh -i /keys/agent-claude agent@servidor
```

A saída é enviada em streaming (`>>`), depois a resposta JSON final (`>>>`):

```
>> web: running
>> api: running
>> worker: stopped
>>> {"command":"agent status","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

O agente pode analisar as linhas `>>` (saída padrão em streaming), detetar que `worker` está parado, e decidir agir em conformidade. A resposta `>>>` confirma o código de retorno.

## Modo sessão

Para evitar abrir uma conexão SSH por comando, o agente pode utilizar o modo sessão:

```bash
{
  echo "+ auth token=agent-claude proof=..."
  echo "+ session keepalive"
  echo "agent status"
  echo "."
  echo "agent logs service=worker"
  echo "."
  echo "."   # bloco vazio = fim de sessão
} | ssh -i /keys/agent-claude agent@servidor
```

Cada comando é seguido de `.` (fim de bloco). Um `.` sem comando anterior assinala o fim da sessão. O modo sessão permite enviar vários comandos numa única conexão SSH, com um timeout global configurável (`timeout_session`).

## Boas práticas

1. **Princípio do menor privilégio**: comece por `read`, eleve por token apenas se necessário
2. **Ações atómicas**: cada ação faz uma única coisa. O agente compõe as ações entre si
3. **Nomes explícitos**: os nomes de domínios e ações são visíveis via `help` — torne-os compreensíveis
4. **Tags de visibilidade**: isole as ações do agente com tags dedicados
5. **Limites de saída**: configure `max_stdout_chars` para evitar que o agente receba volumes demasiado grandes
6. **Logs**: monitorize os logs para detetar utilizações anómalas

---

**Seguinte**: [Integração CI/CD](@/guides/ci-cd.md) — automatizar as implementações via SSH-Frontière.
