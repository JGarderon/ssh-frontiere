# SSH Frontière v3.0.0

**Website**: [pages.nothus.fr/ssh-frontiere](https://pages.nothus.fr/ssh-frontiere/pt/)

[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.md)

Shell de login SSH restrita escrita em Rust — um único ponto de entrada seguro para todas as ligações SSH de entrada num servidor.

O SSH Frontière substitui a shell padrão (`/bin/bash`) em `/etc/passwd` e atua como um **despachante seguro**: valida cada comando SSH contra uma lista branca TOML, aplica controlo de acesso RBAC a 3 níveis e devolve resultados como JSON estruturado através de um protocolo baseado em cabeçalhos no stdin/stdout.

## Propósito

O SSH Frontière é um **componente de segurança** concebido para contas de serviço SSH:

- **Runners CI/CD** (Forgejo Actions, GitHub Actions): operações de infraestrutura a partir de contentores
- **Agentes de IA** (Claude Code, etc.): acesso controlado ao servidor com níveis de confiança
- **Manutenção automatizada**: cópias de segurança, deployments, healthchecks

O programa é **síncrono e one-shot**: o SSH cria um novo processo para cada ligação, o despachante valida e executa, depois termina. Sem daemon, sem async, sem Tokio.

## Instalação

### Pré-requisitos

- Rust 1.70+ com o target `x86_64-unknown-linux-musl`
- `make` (opcional, para atalhos)

### Compilação

```bash
# Via make
make release

# Ou diretamente
cargo build --release --target x86_64-unknown-linux-musl
```

O binário estático resultante (`target/x86_64-unknown-linux-musl/release/ssh-frontiere`, ~1-2 MB) pode ser implantado sem dependências do sistema.

```bash
sudo cp target/x86_64-unknown-linux-musl/release/ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere
```

## Configuração TOML

Ficheiro padrão: `/etc/ssh-frontiere/config.toml`.
Substituição: `--config <caminho>` ou variável de ambiente `SSH_FRONTIERE_CONFIG`.

### Exemplo completo

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300          # Timeout padrão (segundos)
default_level = "read"         # Nível RBAC padrão
mask_sensitive = true           # Mascarar argumentos sensíveis nos logs
max_stdout_chars = 65536       # Limite do stdout capturado
max_stderr_chars = 16384       # Limite do stderr capturado
max_output_chars = 131072      # Limite absoluto global
timeout_session = 3600         # Timeout do keepalive de sessão (segundos)
max_auth_failures = 3          # Tentativas de autenticação antes do bloqueio
log_comments = false           # Registar comentários do cliente
ban_command = ""               # Comando de bloqueio de IP (ex: "/usr/sbin/iptables -A INPUT -s {ip} -j DROP")

# --- Autenticação RBAC (opcional) ---

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Segredo codificado em Base64 com prefixo b64:
level = "ops"                                # Nível concedido por este token

[auth.tokens.admin-deploy]
secret = "b64:c2VjcmV0LWFkbWluLWRlcGxveQ=="
level = "admin"

# --- Domínios e ações ---

[domains.forgejo]
description = "Infraestrutura da forge Git"

[domains.forgejo.actions.backup-config]
description = "Fazer cópia de segurança da configuração do Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.deploy]
description = "Fazer deploy de uma versão"
level = "admin"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {version}"

[[domains.forgejo.actions.deploy.args]]
name = "version"
type = "enum"
values = ["latest", "stable", "canary"]

[domains.infra]
description = "Infraestrutura do servidor"

[domains.infra.actions.healthcheck]
description = "Verificação de disponibilidade"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"
args = []

[domains.infra.actions.set-password]
description = "Alterar palavra-passe do serviço"
level = "admin"
timeout = 30
execute = "sudo /usr/local/bin/set-password.sh {password}"

[[domains.infra.actions.set-password.args]]
name = "password"
type = "string"
sensitive = true    # Mascarado nos logs quando mask_sensitive = true
```

### Tipos de argumentos

| Tipo | Descrição | Validação |
|------|-----------|-----------|
| `string` | Texto livre | Máx. 256 caracteres |
| `enum` | Valor de uma lista | Deve corresponder a um valor em `values` |

### Marcadores em `execute`

- `{domain}`: substituído pelo nome do domínio (sempre disponível)
- `{arg_name}`: substituído pelo valor do argumento correspondente

## Deployment

### 1. Shell de login (`/etc/passwd`)

```bash
# Criar a conta de serviço
sudo useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

O programa será invocado diretamente pelo `sshd` como shell de login.

### 2. Chaves SSH com `authorized_keys`

```
# ~forge-runner/.ssh/authorized_keys

# Chave do runner CI (nível ops)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-key

# Chave de monitorização (nível apenas leitura)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitor-key

# Chave de administrador (nível admin)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

A opção `command=` força a execução do ssh-frontiere com o `--level` escolhido, independentemente do comando enviado pelo cliente. A opção `restrict` desativa o reencaminhamento de portas, reencaminhamento de agente, PTY e X11.

### 3. Sudoers (camada 3)

```
# /etc/sudoers.d/ssh-frontiere
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/set-password.sh
```

Apenas os comandos listados na lista branca TOML **e** autorizados no sudoers podem ser executados com privilégios elevados.

## Protocolo de Cabeçalhos

O SSH Frontière utiliza um protocolo de texto no stdin/stdout com 4 prefixos (ADR 0006).

### Prefixos

| Prefixo | Função | Direção |
|---------|--------|---------|
| `+` | **Configurar**: diretivas (`capabilities`, `challenge`, `auth`, `session`) | bidirecional |
| `#` | **Comentário**: informação, banner, mensagens | bidirecional |
| `$` | **Comando**: comando a executar | cliente → servidor |
| `>` | **Responder**: resposta JSON | servidor → cliente |

### Fluxo de ligação

```
CLIENTE                             SERVIDOR
  |                                    |
  |  <-- banner + capabilities -----  |   # ssh-frontiere 3.0.0
  |  <-- nonce de desafio ----------  |   + capabilities rbac, session, help
  |                                    |   + challenge nonce=a1b2c3...
  |                                    |   # type "help" for available commands
  |                                    |
  |  --- +auth (opcional) -------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (opcional) ----->   |   + session keepalive
  |  --- # comentário (opt.) ----->   |   # client-id: forgejo-runner-12
  |  --- linha vazia ------------->   |   (fim dos cabeçalhos)
  |                                    |
  |  --- domínio ação [args] ------->  |   forgejo backup-config
  |  --- . ------------------------>  |   . (fim do bloco de comando)
  |  <-- >> stdout (streaming) -----  |   >> Backup completed
  |  <-- >>> resposta JSON ---------  |   >>> {"command":"forgejo backup-config","status_code":0,...}
  |                                    |
```

### Resposta JSON (4 campos)

Cada comando produz uma resposta `>>>` com um objeto JSON:

```json
{
  "command": "forgejo backup-config",
  "status_code": 0,
  "status_message": "executed",
  "stdout": null,
  "stderr": null
}
```

- `stdout`/`stderr` = `null`: a saída foi transmitida via prefixos `>>` / `>>!`
- `status_code` = 0: sucesso (código de saída do processo filho em passthrough)

### Códigos de saída

| Código | Significado |
|--------|-------------|
| 0 | Sucesso |
| 1-127 | Código de saída do comando filho (passthrough) |
| 128 | Comando rejeitado |
| 129 | Erro de configuração |
| 130 | Timeout |
| 131 | Nível RBAC insuficiente |
| 132 | Erro de protocolo |
| 133 | Body stdin fechado prematuramente |

## Exemplos Concretos

### Modo one-shot

```bash
# Pipe simples:
{
  echo "infra healthcheck"
  echo "."
} | ssh forge-runner@server
```

### Modo sessão (keepalive)

O modo sessão permite enviar múltiplos comandos numa única ligação SSH:

```bash
{
  echo "+ session keepalive"
  echo "infra healthcheck"
  echo "."
  echo "forgejo backup-config"
  echo "."
  echo "exit"
  echo "."
} | ssh forge-runner@server
```

O servidor responde com uma linha JSON `>>>` para cada comando.

### Autenticação RBAC (elevação de nível)

Um cliente com `--level=read` pode elevar-se para `ops` ou `admin` via challenge-response:

```bash
{
  echo "+ auth token=runner-ci proof=<sha256-hex>"
  echo "forgejo backup-config"    # Requer ops, autorizado via token
  echo "."
} | ssh forge-runner@server
```

O `proof` é `SHA-256(secret)` quando `challenge_nonce = false`, ou `SHA-256(XOR(secret || nonce, secret))` quando `challenge_nonce = true`. O nível efetivo é `max(--level, token.level)`.

### Descoberta (help / list)

```bash
# Lista completa de comandos acessíveis
{ echo "help"; echo "."; } | ssh forge-runner@server

# Detalhes do domínio
{ echo "help forgejo"; echo "."; } | ssh forge-runner@server

# Lista curta (domínio + ação + descrição, JSON)
{ echo "list"; echo "."; } | ssh forge-runner@server
```

Os comandos `help` e `list` apenas mostram ações acessíveis no nível efetivo do cliente.

## Segurança

### Três camadas de defesa em profundidade

| Camada | Mecanismo | Proteção |
|--------|-----------|---------|
| 1 | `command=` + `restrict` em `authorized_keys` | Força `--level`, bloqueia forwarding/PTY |
| 2 | `ssh-frontiere` (shell de login) | Valida o comando contra a lista branca TOML |
| 3 | Lista branca `sudo` em sudoers | Restringe comandos de sistema com privilégios |

Mesmo que um atacante contorne a camada 1 (chave comprometida), a camada 2 bloqueia qualquer comando fora da lista branca. A camada 3 limita os privilégios do sistema.

### Analisador gramatical, não lista negra

**O ssh-frontiere não é uma shell.** A segurança baseia-se num **analisador gramatical**, não na filtragem de caracteres.

- A gramática esperada é `domain action [args]` — tudo o que não corresponder a esta estrutura é rejeitado
- Caracteres especiais (`|`, `;`, `&`, `$`, etc.) entre aspas são **conteúdo** do argumento, não sintaxe de shell — são válidos
- Não existem "caracteres proibidos" — existe uma gramática, e tudo o que não a respeita é rejeitado
- `std::process::Command` executa diretamente sem intermediário de shell — a injeção é estruturalmente impossível

### O que o programa NUNCA faz

- Invocar uma shell (`/bin/bash`, `/bin/sh`)
- Aceitar pipes, redirecionamentos ou encadeamentos (`|`, `>`, `&&`, `;`)
- Executar um comando não listado na lista branca
- Fornecer acesso a um TTY interativo

### Proteções adicionais

- **Timeout** por comando com kill do grupo de processos (SIGTERM depois SIGKILL)
- **Bloqueio** após N tentativas de autenticação falhadas (configurável, padrão: 3)
- **Bloqueio de IP** opcional via comando externo configurável (`ban_command`)
- **Mascaramento** de argumentos sensíveis em logs JSON
- **Limites de tamanho** na saída capturada (stdout, stderr)
- **Nonce anti-replay** regenerado após cada autenticação de sessão bem-sucedida
- **env_clear()** em processos filhos (apenas `PATH` é preservado)

## Testes

```bash
# Testes unitários e de integração
make test

# Testes SSH end-to-end (requer Docker)
make e2e

# Lints (fmt + clippy)
make lint

# Auditoria de segurança de dependências
make audit
```

Os testes E2E (`make e2e`) iniciam um ambiente Docker Compose com servidor e cliente SSH, depois executam cenários que cobrem o protocolo (PRO-*), autenticação (AUT-*), sessões (SES-*), segurança (SEC-*), robustez (ROB-*) e logging (LOG-*).

## Contribuir

As contribuições são bem-vindas! Consulte o [guia de contribuição](CONTRIBUTING.md) para mais detalhes.

## Licença

Este projeto é distribuído sob a [Licença Pública da União Europeia (EUPL-1.2)](LICENSE.md).

Copyright (c) Julien Garderon, 2024-2026
