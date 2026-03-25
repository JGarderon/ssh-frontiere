+++
title = "Apresentação"
description = "Descobrir SSH-Frontière: o que é, por que existe, como funciona"
date = 2026-03-24
weight = 1
+++

# Apresentação do SSH-Frontière

## O problema

Num servidor Linux, as contas de serviço SSH (runners CI, agentes IA, scripts de manutenção) utilizam geralmente `/bin/bash` como shell de ligação. Isto coloca vários problemas:

- **Nenhum controlo**: o cliente SSH pode executar qualquer comando
- **Sem auditoria**: os comandos executados não são registados de forma estruturada
- **Sem granularidade**: um script que precisa de ler um estado tem os mesmos direitos que um script de implementação

As soluções clássicas (`authorized_keys` com `command=`, scripts wrapper bash, bastiões SSH) têm cada uma as suas limitações: frágeis, difíceis de auditar, ou sobredimensionadas para a necessidade.

## O que faz SSH-Frontière

SSH-Frontière é um **shell de login de substituição**. Coloca-se entre `sshd` e os comandos do sistema:

```
Cliente SSH
    |
    v
sshd (autenticação por chave)
    |
    v
ssh-frontiere (login shell)
    |
    ├── Valida o comando contra a configuração TOML
    ├── Verifica o nível de acesso (read / ops / admin)
    ├── Executa o comando autorizado
    └── Devolve o resultado em JSON estruturado
```

Cada conexão SSH cria um novo processo `ssh-frontiere` que:

1. Apresenta um banner e as capacidades do servidor
2. Lê os cabeçalhos do cliente (autenticação, modo sessão)
3. Lê o comando (`domínio ação [argumentos]`, texto simples)
4. Valida contra a whitelist TOML
5. Executa se autorizado, rejeita caso contrário
6. Devolve uma resposta JSON e termina

O programa é **síncrono e efémero**: sem daemon, sem serviço, sem estado persistente.

## O que SSH-Frontière não faz

- **Não é um bastião SSH**: sem proxy, sem reencaminhamento de conexão para outros servidores
- **Não é um gestor de chaves**: a gestão de chaves SSH permanece no `authorized_keys` e no `sshd`
- **Não é um shell**: sem interpretação de comandos, sem pipe, sem redirecionamento, sem interatividade
- **Não é um daemon**: executa e termina a cada conexão

## Casos de utilização concretos

### Automatização CI/CD

Um runner Forgejo Actions implementa uma aplicação via SSH:

```bash
# O runner envia o comando via SSH
{
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@servidor
```

SSH-Frontière verifica que o runner tem o nível `admin`, que a ação `deploy` existe no domínio `forgejo`, que o argumento `version=stable` é um valor autorizado, e depois executa o script de implementação configurado.

### Agentes IA

Um agente Claude Code atua num servidor com direitos limitados:

```bash
# O agente descobre os comandos disponíveis
{ echo "list"; echo "."; } | ssh agent-ia@servidor

# O agente executa uma ação específica
{ echo "infra healthcheck"; echo "."; } | ssh agent-ia@servidor
```

O agente só tem acesso às ações de nível `read` configuradas para ele. Os comandos `help` e `list` permitem-lhe descobrir as ações disponíveis e os seus parâmetros — formato JSON, nativamente analisável.

### Manutenção automatizada

Scripts cron executam salvaguardas via SSH:

```bash
# Salvaguarda noturna
{ echo "forgejo backup-config"; echo "."; } | ssh backup@servidor

# Notificação após implementação
{ echo 'notify send message="Implementação concluída"'; echo "."; } | ssh notify@servidor
```

### Notificações

Acionar notificações (Slack, Olvid, email) como ações SSH-Frontière padrão:

```bash
{ echo 'notify slack channel=ops message="Build OK"'; echo "."; } | ssh notify@servidor
```

## Porquê SSH-Frontière em vez de...

### ...scripts bash no `authorized_keys`?

A opção `command=` no `authorized_keys` permite forçar um comando, mas:
- Um só script por chave — sem granularidade
- Sem validação de argumentos
- Sem níveis de acesso
- Sem logging estruturado
- O script bash pode conter vulnerabilidades (injeção, globbing)

SSH-Frontière oferece uma configuração declarativa, RBAC, logging JSON e um parser gramatical que elimina as injeções.

### ...um bastião SSH (Teleport, Boundary)?

Os bastiões SSH são concebidos para gerir o acesso de **pessoas** a servidores:
- Pesados de implementar e de manter
- Sobredimensionados para contas de serviço
- Modelo de ameaça diferente (utilizador interativo vs script automatizado)

SSH-Frontière é um componente leve (~1 Mo) concebido para as **contas de serviço**: sem sessão interativa, sem proxy, apenas validação de comandos.

### ...`sudo` sozinho?

`sudo` controla a elevação de privilégios, mas:
- Não controla o que o cliente SSH pode *pedir*
- Sem protocolo estruturado (entradas/saídas JSON)
- Sem logging integrado ao nível do comando SSH

SSH-Frontière e `sudo` são complementares: SSH-Frontière valida o comando de entrada, `sudo` controla os privilégios de sistema. É a camada 2 e a camada 3 da defesa em profundidade.

## O interesse do produto

SSH-Frontière traz uma **governação declarativa** dos acessos SSH de serviço:

1. **Tudo está num ficheiro TOML**: os domínios, as ações, os argumentos, os níveis de acesso. Sem lógica dispersa em scripts.

2. **Implementação instantânea**: como toda a configuração está centralizada num único ficheiro TOML, implementar uma nova versão é trivial. Cada conexão SSH cria um novo processo que relê a configuração — as alterações são tidas em conta assim que termina a sessão em curso ou imediatamente para qualquer novo cliente.

3. **Zero trust por predefinição**: nada é executado sem estar explicitamente configurado. Sem shell, sem injeção possível.

4. **Auditável**: cada tentativa (autorizada ou rejeitada) é registada em JSON estruturado com timestamp, comando, argumentos, nível, resultado.

5. **Compatível com LLM**: os agentes IA podem descobrir as ações disponíveis via `help`/`list`, e interagir via um protocolo JSON estruturado — não é necessário analisar texto livre.

6. **Europeu e open source**: licença EUPL-1.2, desenvolvido em França, sem dependência de um ecossistema proprietário.

---

Para aprofundar: [Instalação](@/installation/_index.md) | [Arquitetura](@/architecture.md) | [Segurança](@/securite.md) | [Alternativas](@/alternatives.md)
