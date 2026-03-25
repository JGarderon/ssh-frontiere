+++
title = "Arquitetura"
description = "Conceção técnica do SSH-Frontière: linguagem, módulos, protocolo, dependências"
date = 2026-03-24
weight = 3
+++

# Arquitetura e conceção

## Porquê Rust

SSH-Frontière é escrito em Rust por três razões:

1. **Segurança de memória**: sem buffer overflow, sem use-after-free, sem null pointer. Para um componente de segurança que funciona como login shell, isto é crítico.

2. **Binário estático**: compilado com o alvo `x86_64-unknown-linux-musl` (outros alvos possíveis sem garantia de funcionamento), o binário tem ~1 Mo e não tem qualquer dependência de sistema. Copia-se para o servidor e está pronto.

3. **Desempenho**: o programa arranca, valida, executa e termina em milissegundos. Sem runtime, sem garbage collector, sem JIT.

## Síncrono e efémero

SSH-Frontière é um programa **síncrono e one-shot**. Sem daemon, sem async, sem Tokio.

O ciclo de vida é simples:
1. `sshd` autentica a conexão SSH por chave
2. `sshd` faz fork e executa `ssh-frontiere` como login shell
3. `ssh-frontiere` valida e executa o comando
4. O processo termina

Cada conexão SSH cria um novo processo. Sem estado partilhado entre conexões, sem problemas de concorrência.

## Estrutura do código

O código está organizado em módulos com responsabilidades claras:

| Módulo | Responsabilidade |
|--------|------------------|
| `main.rs` | Ponto de entrada, aplanamento dos argumentos, chamada ao orquestrador |
| `orchestrator.rs` | Fluxo principal: banner, cabeçalhos, comando, resposta, ciclo de sessão |
| `config.rs` | Estruturas de configuração TOML, validação fail-fast |
| `protocol.rs` | Protocolo de cabeçalhos: parser, banner, auth, sessão, body |
| `crypto.rs` | SHA-256 (implementação FIPS 180-4), base64, nonce, challenge-response |
| `dispatch.rs` | Parsing de comando (aspas, `key=value`), resolução, RBAC |
| `chain_parser.rs` | Parser de cadeias de comandos (operadores `;`, `&`, `\|`) |
| `chain_exec.rs` | Execução das cadeias: sequência estrita (`;`), permissiva (`&`), recuperação (`\|`) |
| `discovery.rs` | Comandos `help` e `list`: descoberta dos domínios e ações |
| `logging.rs` | Logging JSON estruturado, mascaramento de argumentos sensíveis |
| `output.rs` | Resposta JSON, códigos de saída |
| `lib.rs` | Exposição de `crypto` para o binário proof e helpers de fuzz |

Cada módulo tem o seu ficheiro de testes (`*_tests.rs`) no mesmo diretório.

Um binário auxiliar `proof` (`src/bin/proof.rs`) permite calcular os proofs de autenticação para os testes E2E e a integração com clientes.

## Protocolo de cabeçalhos

SSH-Frontière utiliza um protocolo de texto em stdin/stdout. Os prefixos diferem consoante a direção:

**Cliente para servidor (stdin):**

| Prefixo | Função |
|---------|--------|
| `+ ` | **Configura**: diretivas (`auth`, `session`, `body`) |
| `# ` | **Comenta**: ignorados pelo servidor |
| *(texto simples)* | **Comando**: `domínio ação [argumentos]` |
| `.` *(só numa linha)* | **Fim de bloco**: termina um bloco de comando |

**Servidor para cliente (stdout):**

| Prefixo | Função |
|---------|--------|
| `#> ` | **Comenta**: banner, mensagens informativas |
| `+> ` | **Configura**: capabilities, challenge nonce |
| `>>> ` | **Responde**: resposta JSON final |
| `>> ` | **Stdout**: saída padrão em streaming (ADR 0011) |
| `>>! ` | **Stderr**: saída de erro em streaming |

### Fluxo de conexão

```
CLIENTE                                 SERVIDOR
  |                                        |
  |  <-- banner + capabilities ----------  |   #> ssh-frontiere 0.1.0
  |                                        |   +> capabilities rbac, session, help, body
  |                                        |   +> challenge nonce=a1b2c3...
  |                                        |   #> type "help" for available commands
  |                                        |
  |  --- +auth (opcional) ------------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (opcional) ---------->   |   + session keepalive
  |                                        |
  |  --- comando (texto simples) ------>   |   forgejo backup-config
  |  --- fim de bloco ----------------->   |   .
  |  <-- streaming stdout -------------   |   >> Backup completed
  |  <-- resposta JSON final ----------   |   >>> {"status_code":0,"status_message":"executed",...}
  |                                        |
  |  (se session keepalive)                |
  |  --- comando 2 ------------------->   |   infra healthcheck
  |  --- fim de bloco ----------------->   |   .
  |  <-- resposta JSON 2 -------------   |   >>> {"status_code":0,...}
  |  --- fim de sessão (bloco vazio) ->   |   .
  |  <-- session closed ---------------   |   #> session closed
```

### Resposta JSON

Cada comando produz uma resposta JSON final numa única linha, prefixada por `>>>`. A saída padrão e de erro são enviadas em streaming via `>>` e `>>!`:

```
>> Backup completed
>>> {"command":"forgejo backup-config","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` na resposta JSON final: a saída foi enviada em streaming via `>>` e `>>!`
- Para comandos não executados (rejeição, erro de config), `stdout` e `stderr` são também `null`

### Protocolo body

O cabeçalho `+body` permite transmitir conteúdo multilinha para o processo filho via stdin. Quatro modos de delimitação:

- `+body`: lê até uma linha contendo apenas `.` (ponto)
- `+body size=N`: lê exatamente N octetos
- `+body stop="DELIMITADOR"`: lê até uma linha contendo o delimitador
- `+body size=N stop="DELIMITADOR"`: o primeiro delimitador alcançado (tamanho ou marcador) termina a leitura

## Configuração TOML

O formato de configuração é TOML declarativo. Escolha documentada na ADR 0001:

- **Porquê TOML**: legível por humanos, tipagem nativa, padrão no ecossistema Rust, sem indentação significativa (ao contrário do YAML), mais expressivo que JSON para configuração.
- **Porquê não YAML**: indentação significativa fonte de erros, tipos implícitos perigosos (`on`/`off` -> booleano), especificação complexa.
- **Porquê não JSON**: sem comentários, verboso, não concebido para configuração humana.

A configuração é **validada no carregamento** (fail-fast): sintaxe TOML, completude dos campos, coerência dos placeholders, pelo menos um domínio, pelo menos uma ação por domínio, valores enum não vazios.

## Política de dependências

SSH-Frontière tem uma política de **zero dependências não vitais**. Cada crate externa deve ser justificada por uma necessidade real.

### Dependências atuais

3 dependências diretas, ~20 dependências transitivas:

| Crate | Utilização |
|-------|------------|
| `serde` + `serde_json` | Serialização JSON (logging, respostas) |
| `toml` | Carregamento da configuração TOML |

### Matriz de avaliação

Antes de adicionar uma dependência, é avaliada em 8 critérios ponderados (nota /5): licença (eliminatório), governação (x3), comunidade (x2), frequência de atualização (x2), tamanho (x3), dependências transitivas (x3), funcionalidades (x2), não-aprisionamento (x1). Pontuação mínima: 3.5/5.

### Auditoria

- `cargo deny` verifica as licenças e as vulnerabilidades conhecidas
- `cargo audit` procura falhas na base RustSec
- Fontes autorizadas: crates.io apenas

## Como o projeto foi concebido

SSH-Frontière foi desenvolvido em fases sucessivas (1 a 9, com fases intermédias 2.5 e 5.5), pilotado por agentes Claude Code com uma metodologia TDD sistemática:

| Fase | Conteúdo |
|------|----------|
| 1 | Dispatcher funcional, config TOML, RBAC 3 níveis |
| 2 | Configuração de produção, scripts de operações |
| 2.5 | SHA-256 FIPS 180-4, BTreeMap, timeout gracioso |
| 3 | Protocolo de cabeçalhos unificado, auth challenge-response, sessões |
| 4 | Testes E2E SSH Docker, limpeza de código, integração forge |
| 5 | Tags de visibilidade, filtragem horizontal por tokens |
| 5.5 | Nonce opcional, argumentos nomeados, binário proof (inclui a fase 6, fundida) |
| 7 | Guia de configuração, dry-run `--check-config`, help sem prefixo |
| 8 | Tipos de erro estruturados, clippy pedantic, cargo-fuzz, proptest |
| 9 | Protocolo body, argumentos livres, max_body_size, código de saída 133 |

O projeto foi concebido por:
- **Julien Garderon** (BO): conceito, especificações funcionais, escolha Rust, nome do projeto
- **Claude superviseur** (PM/Tech Lead): análise técnica, arquitetura
- **Agentes Claude Code**: implementação, testes, documentação

Onde o humano e a máquina trabalham juntos, melhor, mais depressa, com mais segurança.
