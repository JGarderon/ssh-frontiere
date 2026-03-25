+++
title = "Segurança"
description = "Modelo de segurança, garantias e limitações do SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Segurança

SSH-Frontière é um **componente de segurança**. A sua razão de ser é restringir o que as conexões SSH de entrada podem fazer. Esta página documenta o modelo de segurança, o que foi implementado e o que não é garantido.

## Modelo de segurança

### Princípio fundamental: deny by default

Nada é executado sem estar explicitamente configurado. Se um comando não estiver na whitelist TOML, é rejeitado. Não existe modo permissivo, não existe fallback para um shell.

### Três camadas de defesa em profundidade

| Camada | Mecanismo | Proteção |
|--------|-----------|----------|
| 1 | `command=` + `restrict` no `authorized_keys` | Força o nível de acesso, bloqueia forwarding/PTY |
| 2 | SSH-Frontière (login shell) | Valida o comando contra a whitelist TOML |
| 3 | `sudo` whitelist nos sudoers | Restringe os comandos de sistema privilegiados |

Mesmo que um atacante comprometa uma chave SSH (camada 1), só pode executar os comandos autorizados na whitelist TOML (camada 2). Mesmo que contorne a camada 2, só pode elevar os seus privilégios para os comandos autorizados nos sudoers (camada 3).

### Parser gramatical, não lista negra

SSH-Frontière **não é um shell**. A segurança não se baseia numa filtragem de caracteres (sem lista negra de `|`, `;`, `&`), mas num **parser gramatical**.

A gramática esperada é `domínio ação [key=value ...]`. Tudo o que não respeita esta estrutura é rejeitado. Os caracteres especiais entre aspas são conteúdo de argumento, não sintaxe — são válidos.

`std::process::Command` executa diretamente, sem passar por um shell intermédio. A injeção de comandos é **estruturalmente impossível**.

### Determinismo face aos agentes IA

Este funcionamento é **determinístico**: um dado comando produz sempre o mesmo resultado de validação, independentemente do contexto. É uma propriedade essencial quando se trabalha com agentes de IA, cuja natureza é justamente o **indeterminismo** — um modelo pode ser enviesado, ou a cadeia de produção do agente pode estar corrompida, visando os shells para recuperar informações adicionais ou exfiltrar segredos. Com SSH-Frontière, um agente comprometido não pode contornar a whitelist, não pode injetar comandos num shell, e não pode aceder a recursos não configurados. É **estruturalmente impossível**.

## O que foi implementado

### Linguagem Rust

SSH-Frontière é escrito em Rust, o que elimina as classes de vulnerabilidades mais comuns em programas de sistema:
- Sem buffer overflow
- Sem use-after-free
- Sem null pointer dereference
- Sem `unsafe` no código (proibido pela configuração de lints no `Cargo.toml`: `unsafe_code = "deny"`)

### 399 testes cargo + 72 cenários E2E SSH

O projeto é coberto por **399 testes cargo** e **72 cenários E2E SSH** adicionais:

| Tipo | Número | Descrição |
|------|--------|-----------|
| Testes unitários | ~340 | Cada módulo testa independentemente (10 ficheiros `*_tests.rs`) |
| Testes de integração | 50 | Cenários stdio completos (execução do binário) |
| Testes de conformidade | 1 (6 cenários) | Validação do contrato de interface JSON (ADR 0003) |
| Testes proptest | 8 | Testes de propriedades (fuzzing guiado por restrições) |
| **Total cargo** | **399** | |
| Cenários E2E SSH | 72 | Docker Compose com servidor SSH real |
| Harnesses cargo-fuzz | 9 | Fuzzing não guiado (mutações aleatórias) |

Os testes E2E SSH cobrem o protocolo completo, a autenticação, as sessões, a segurança, a robustez e o logging. São executados num ambiente Docker Compose com um servidor SSH real.

### Auditoria das dependências

- `cargo deny` em CI: verifica as licenças e as vulnerabilidades conhecidas (base RustSec)
- `cargo audit`: auditoria de segurança das dependências
- `cargo clippy` em modo pedantic: 0 warnings autorizados
- Apenas 3 dependências diretas: `serde`, `serde_json`, `toml` — todas amplamente auditadas pela comunidade Rust

### Controlo de acesso RBAC

Três níveis de confiança hierárquicos:

| Nível | Utilização | Exemplos |
|-------|------------|----------|
| `read` | Apenas consulta | healthcheck, status, list |
| `ops` | Operações correntes | backup, deploy, restart |
| `admin` | Todas as ações | configuração, dados sensíveis |

Cada ação tem um nível exigido. Cada conexão SSH tem um nível efetivo (via `--level` no `authorized_keys` ou via autenticação por token).

### Tags de visibilidade

Em complemento ao RBAC vertical, **tags** permitem uma filtragem horizontal: um token com o tag `forgejo` só vê as ações marcadas com `forgejo`, mesmo que tenha o nível `ops`.

### Autenticação por token

Dois modos de autenticação:

- **Modo simples** (`challenge_nonce = false`): challenge-response `SHA-256(secret)` — o cliente prova que conhece o segredo
- **Modo nonce** (`challenge_nonce = true`): challenge-response `SHA-256(XOR_encrypt(secret || nonce, secret))` com o nonce enviado pelo servidor. O nonce é regenerado após cada autenticação bem-sucedida, impedindo a reprodução de um proof intercetado

### Proteções adicionais

- **Timeout** por comando com kill do process group (SIGTERM depois SIGKILL)
- **Lockout** após N tentativas de autenticação falhadas (configurável, padrão: 3)
- **Ban IP** opcional via comando externo configurável
- **Mascaramento** de argumentos sensíveis nos logs (SHA-256)
- **Limite de tamanho** nas saídas capturadas (stdout, stderr)
- **Limpeza de ambiente**: `env_clear()` nos processos filhos, apenas `PATH` e `SSH_FRONTIERE_SESSION` são injetados

## O que não é garantido

Nenhum software é perfeito. Eis as limitações conhecidas e documentadas:

### Contador XOR 8 bits

A implementação criptográfica utiliza um contador XOR com um keystream limitado a 8192 bytes. É suficiente para o uso atual (proofs SHA-256 de 64 caracteres), mas não foi concebido para cifrar grandes volumes.

### Fuga de comprimento na comparação

A comparação em tempo constante pode revelar o comprimento dos valores comparados. Na prática, os proofs SHA-256 têm sempre 64 caracteres, o que torna esta fuga negligenciável.

### Rate limiting por conexão

O contador de tentativas de autenticação é local a cada conexão SSH. Um atacante pode abrir N conexões e ter N x `max_auth_failures` tentativas. Recomendação: complementar com fail2ban, `sshd MaxAuthTries`, ou regras iptables.

### Reportar uma vulnerabilidade

**Não reporte vulnerabilidades através das issues públicas.** Contacte diretamente o mantenedor para uma divulgação responsável. O processo está descrito no [guia de contribuição](@/contribuer.md).

## Dependências

SSH-Frontière tem uma política estrita de dependências mínimas. Cada crate externa é avaliada segundo uma matriz ponderada (licença, governação, comunidade, tamanho, dependências transitivas).

| Crate | Versão | Utilização | Justificação |
|-------|--------|------------|--------------|
| `serde` | 1.x | Serialização/desserialização | Standard de facto Rust, necessário para JSON e TOML |
| `serde_json` | 1.x | Respostas JSON | Formato de saída do protocolo |
| `toml` | 0.8.x | Carregamento da configuração | Formato padrão Rust para configuração |

Dependência de desenvolvimento: `proptest` (testes de propriedades apenas, não incluída no binário final).

Fontes autorizadas: **crates.io apenas**. Nenhum repositório git externo autorizado. Política verificada por `cargo deny`.
