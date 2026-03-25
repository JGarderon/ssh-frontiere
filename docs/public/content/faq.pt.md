+++
title = "FAQ"
description = "Perguntas frequentes sobre SSH-Frontière"
date = 2026-03-24
weight = 5
+++

# Perguntas frequentes

## O que é exatamente SSH-Frontière?

Um **shell de login de substituição** escrito em Rust. Instala-se no lugar de `/bin/bash` no `/etc/passwd` para uma conta de serviço. Cada conexão SSH passa por SSH-Frontière que valida o comando contra um ficheiro de configuração TOML antes de o executar.

## É um bastião SSH?

Não. Um bastião SSH (Teleport, Boundary) é um **proxy** que reencaminha conexões para outros servidores. SSH-Frontière não faz reencaminhamento — controla o que é executado **no servidor onde está instalado**.

Os bastiões gerem o acesso de pessoas a um parque de servidores. SSH-Frontière gere o acesso de **contas de serviço** (runners CI, agentes IA, scripts) a ações específicas num servidor.

## Substitui o `sudo`?

Não, é complementar. SSH-Frontière controla o que o cliente SSH **pode pedir** (camada 2). `sudo` controla os privilégios de sistema **necessários à execução** (camada 3). Os dois combinam-se para uma defesa em profundidade.

## Pode ser utilizado sem ficheiro TOML?

Não. O ficheiro de configuração é obrigatório. É intencional: tudo é explícito, declarativo e auditável. Sem modo permissivo, sem fallback para um shell.

## O que acontece se a configuração for inválida?

SSH-Frontière valida integralmente a configuração no arranque (fail-fast). Se a configuração for inválida, o programa termina com o código 129 e uma mensagem de erro explícita no registo. Nenhum comando é executado. O cliente SSH, por sua vez, **nunca** vê o detalhe do erro — apenas que o serviço não está disponível. As informações de diagnóstico ficam do lado do servidor.

Pode testar a configuração sem risco:

```bash
ssh-frontiere --check-config --config /etc/ssh-frontiere/config.toml
```

## Como diagnosticar um problema?

Várias ferramentas estão disponíveis:

1. **Validação de config**: `ssh-frontiere --check-config` verifica a sintaxe e a coerência
2. **Comando `help`**: mostra as ações acessíveis ao nível efetivo do cliente
3. **Comando `list`**: versão curta (domínio + ação)
4. **Logs JSON**: cada comando (executado ou rejeitado) é registado com timestamp, comando, argumentos, nível, resultado
5. **Código de saída**: 0 = sucesso, 128 = rejeitado, 129 = erro de config, 130 = timeout, 131 = nível insuficiente, 132 = erro de protocolo, 133 = body stdin fechado prematuramente

## Os agentes IA podem utilizá-lo?

Sim, é um caso de uso de primeira classe. Os comandos `help` e `list` devolvem JSON estruturado, diretamente analisável por um agente. O protocolo de cabeçalhos (prefixos `+`, `#`, `$`, `>`) foi concebido para ser legível por máquinas sem perturbar a leitura humana.

Consulte o [guia de agentes IA](@/guides/agents-ia.md) para a configuração detalhada.

## Quais são as dependências no código-fonte?

3 dependências diretas:

| Crate | Utilização |
|-------|------------|
| `serde` + `serde_json` | Serialização JSON (logs, respostas) |
| `toml` | Carregamento da configuração |

Sem runtime async, sem Tokio, sem framework web. O binário estático tem ~1 Mo.

## Porquê Rust e não Go/Python?

1. **Segurança de memória**: sem buffer overflow, sem use-after-free — crítico para um componente de segurança
2. **Binário estático**: compila com musl, nenhuma dependência de sistema
3. **Desempenho**: arranque em milissegundos, sem runtime
4. **Sem `unsafe`**: proibido pelos lints Cargo (`unsafe_code = "deny"`)

## Porquê TOML e não YAML ou JSON?

- **TOML**: legível, tipado, comentários, padrão Rust, sem indentação significativa
- **YAML**: indentação significativa fonte de erros, tipos implícitos perigosos (`on`/`off` -> booleano)
- **JSON**: sem comentários, verboso, não concebido para configuração humana

A escolha está documentada na ADR 0001.

## Como funciona a autenticação por token?

Dois modos:

1. **Modo simples** (`challenge_nonce = false`): o cliente calcula `SHA-256(secret)` e envia-o como proof
2. **Modo nonce** (`challenge_nonce = true`): o servidor envia um nonce, o cliente calcula `SHA-256(XOR_encrypt(secret || nonce, secret))`

O modo nonce protege contra a reprodução: cada proof é único graças ao nonce.

## Podem ser utilizadas várias chaves SSH?

Sim. Cada chave no `authorized_keys` tem o seu próprio `--level`. Várias chaves podem coexistir com níveis diferentes:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin
```

## Qual é o formato das respostas?

A saída padrão e de erro são enviadas em streaming (prefixos `>>` e `>>!`), depois uma resposta JSON final numa única linha (prefixo `>>>`):

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` no JSON final: a saída foi enviada em streaming
- `status_code` = 0: sucesso (código de saída do processo filho em passthrough)

## Como atualizar SSH-Frontière?

1. Compilar a nova versão (`make release`)
2. Copiar o binário para o servidor (`scp`)
3. Verificar (`ssh user@servidor` + `help`)

Sem migração de dados, sem esquema de base de dados. O ficheiro TOML é versionável com git.

## Como contribuir?

Consulte o [guia de contribuição](@/contribuer.md). Em resumo: abrir uma issue, fork, TDD, pull request, CI verde. As contribuições geradas por IA são aceites.

## Onde encontrar o código-fonte?

O código-fonte está disponível no [repositório GitHub](https://github.com/nothus-forge/ssh-frontiere). Licença [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
