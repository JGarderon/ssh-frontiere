+++
title = "Configuração"
description = "Escrever o ficheiro config.toml do SSH-Frontière"
date = 2026-03-24
weight = 3
+++

# Configuração

SSH-Frontière utiliza um ficheiro TOML para declarar os domínios, ações, níveis de acesso, argumentos e tokens de autenticação.

## Localização

**Caminho por predefinição**: `/etc/ssh-frontiere/config.toml`

**Override** (por ordem de prioridade):
1. `--config <path>` na linha `command=` do `authorized_keys`
2. Variável de ambiente `SSH_FRONTIERE_CONFIG`
3. Caminho por predefinição

**Permissões recomendadas**: `root:forge-runner 640` (adapte o grupo à conta de serviço utilizada).

## Estrutura do ficheiro

```toml
[global]                              # Parâmetros gerais
[domains.<id>]                        # Domínios funcionais
  [domains.<id>.actions.<id>]         # Ações autorizadas
    [domains.<id>.actions.<id>.args]  # Argumentos nomeados (opcional)
[auth]                                # Autenticação RBAC (opcional)
  [auth.tokens.<id>]                  # Tokens com segredo, nível e tags
```

## Secção `[global]`

| Chave | Tipo | Predefinição | Descrição |
|-------|------|--------------|-----------|
| `log_file` | string | **obrigatório** | Caminho do ficheiro de log JSON |
| `default_timeout` | inteiro | `300` | Timeout por predefinição em segundos |
| `max_stdout_chars` | inteiro | `65536` | Limite stdout (64 Ko) |
| `max_stderr_chars` | inteiro | `16384` | Limite stderr (16 Ko) |
| `max_output_chars` | inteiro | `131072` | Hard limit global (128 Ko) |
| `max_stream_bytes` | inteiro | `10485760` | Limite de volume streaming (10 Mo) |
| `timeout_session` | inteiro | `3600` | Timeout de sessão keepalive |
| `max_auth_failures` | inteiro | `3` | Tentativas de auth antes do lockout |
| `ban_command` | string | `""` | Comando de ban IP (placeholder `{ip}`) |
| `log_comments` | bool | `false` | Registar as linhas `#` do cliente |
| `expose_session_id` | bool | `false` | Mostrar o UUID da sessão no banner |

As chaves `log_level`, `default_level` e `mask_sensitive` são aceites pelo parser para compatibilidade com configurações antigas, mas já não são utilizadas.

## Secção `[domains]`

Um **domínio** é um perímetro funcional (ex.: `forgejo`, `infra`, `notify`). Cada domínio contém **ações** autorizadas.

```toml
[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Salvaguarda a configuração"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
max_body_size = 65536       # Limite body (64 Ko, opcional)
```

Cada ação aceita as seguintes chaves: `description` (obrigatório), `level` (obrigatório), `execute` (obrigatório), `timeout` (opcional, override do global), `tags` (opcional), `max_body_size` (opcional, predefinição 65536 octetos — limitado para o protocolo `+body`).

### Níveis de confiança

Hierarquia estrita: `read` < `ops` < `admin`

| Nível | Utilização |
|-------|------------|
| `read` | Consulta: healthcheck, status, list |
| `ops` | Operações correntes: backup, deploy, restart |
| `admin` | Todas as ações + administração |

### Argumentos

Os argumentos são declarados como um dicionário TOML:

```toml
[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
```

| Campo | Tipo | Descrição |
|-------|------|-----------|
| `type` | string | `"enum"` ou `"string"` |
| `values` | lista | Valores autorizados (para `enum`) |
| `default` | string | Valor por predefinição (torna o argumento opcional) |
| `sensitive` | bool | Se `true`, mascara nos logs |
| `free` | bool | Se `true`, aceita qualquer valor sem restrição |

### Placeholders no `execute`

| Placeholder | Descrição |
|-------------|-----------|
| `{domain}` | Nome do domínio (sempre disponível) |
| `{nome_arg}` | Valor do argumento correspondente |

### Tags de visibilidade

Os tags filtram horizontalmente o acesso às ações. Uma ação sem tags é acessível por todos. Uma ação com tags só é acessível pelas identidades cujos tags tenham pelo menos um tag em comum.

```toml
[domains.forgejo.actions.deploy]
# ...
tags = ["forgejo", "deploy"]
```

## Secção `[auth]` (opcional)

A autenticação RBAC permite a elevação de privilégios via challenge-response:

```toml
[auth]
challenge_nonce = false              # true = modo nonce anti-replay

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Segredo em base64
level = "ops"                               # Nível concedido
tags = ["forgejo"]                          # Tags de visibilidade
```

O segredo deve ser prefixado por `b64:` e codificado em base64. Para gerar um segredo:

```bash
echo -n "meu-segredo-aleatorio" | base64
# bWV1LXNlZ3JlZG8tYWxlYXRvcmlv
```

## Validação no carregamento

A configuração é validada integralmente a cada carregamento (fail-fast). Em caso de erro, o programa termina com o código 129. Validações:

- Sintaxe TOML correta
- Pelo menos um domínio, pelo menos uma ação por domínio
- Cada ação tem um `execute` e um `level` válido
- Os placeholders `{arg}` no `execute` correspondem aos argumentos declarados
- Os argumentos enum têm pelo menos um valor autorizado
- Os valores por predefinição estão na lista de valores autorizados
- `max_stdout_chars` e `max_stderr_chars` <= `max_output_chars`

## Exemplo completo

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 3600
max_auth_failures = 3

[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Salvaguarda a configuração Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"

[domains.forgejo.actions.deploy]
description = "Implementação com tag de versão"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.infra]
description = "Infraestrutura do servidor"

[domains.infra.actions.healthcheck]
description = "Verificação de saúde"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[auth]
challenge_nonce = false

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

Para um guia detalhado com todos os casos de utilização, consulte o [guia de configuração completo](https://github.com/nothus-forge/ssh-frontiere/blob/main/docs/references/configuration.md) no repositório.

---

**Seguinte**: [Implementação](@/installation/deploiement.md) — colocar em produção.
