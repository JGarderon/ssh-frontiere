+++
title = "Alternativas"
description = "Comparação do SSH-Frontière com soluções existentes de controlo SSH"
date = 2026-03-24
weight = 4
+++

# Comparação com as alternativas

SSH-Frontière não é a única forma de controlar os acessos SSH. Esta página compara as abordagens existentes para o ajudar a escolher a solução adequada.

## Tabela comparativa

| Critério | `authorized_keys` `command=` | SSH-Frontière | Teleport | Boundary |
|----------|------------------------------|---------------|----------|----------|
| **Tipo** | Opção OpenSSH | Login shell | Bastião SSH | Bastião SSH |
| **Alvo** | Script único por chave | Contas de serviço | Utilizadores humanos | Utilizadores humanos |
| **Granularidade** | 1 comando por chave | RBAC 3 níveis, domínios, ações, argumentos | Papéis, labels, RBAC | Políticas IAM |
| **Logging** | Não estruturado | JSON estruturado por comando | Sessão completa (replay) | Audit trail |
| **Implementação** | Nativo (OpenSSH) | 1 binário + 1 ficheiro TOML | Cluster (auth server, proxy, node) | Cluster (controller, workers) |
| **Dependências** | Nenhuma | 0 dependência de sistema | Base de dados, certificados | Base de dados |
| **Tamanho** | — | ~1 Mo (binário estático) | ~100 Mo | ~100 Mo |
| **Anti-injeção** | Responsabilidade do script | Estrutural (parser gramatical) | N/A (sessão interativa) | N/A (sessão interativa) |
| **Compatível LLM** | Não | Sim (JSON, help, descoberta) | Não | Não |
| **Licença** | OpenSSH (BSD) | EUPL-1.2 | AGPL-3.0 (OSS) / Comercial | BSL 1.1 |

## `authorized_keys` com `command=`

A opção `command=` no `authorized_keys` permite forçar a execução de um script a cada conexão. É a solução mais simples e mais difundida.

### Vantagens

- **Zero instalação**: funcionalidade nativa do OpenSSH
- **Simples** para um caso de uso único (uma chave = um comando)

### Limitações

- **Um só script por chave**: sem granularidade fina. Para N ações diferentes, são necessárias N chaves ou um script bash que analise `$SSH_ORIGINAL_COMMAND`
- **Sem validação de argumentos**: o script recebe uma cadeia bruta e tem de a validar ele próprio — fonte de injeção se mal feito
- **Sem níveis de acesso**: todas as chaves têm os mesmos direitos (ou é necessário codificá-los no script)
- **Sem logging estruturado**: os logs dependem do script
- **Frágil**: um script bash com validação de comandos é difícil de proteger e de manter

### Quando escolher `command=`

- Necessidade simples: uma chave SSH, um comando fixo, sem parâmetros
- Sem exigência de auditoria ou de RBAC

## Teleport

[Teleport](https://goteleport.com/) é um bastião SSH completo com gravação de sessões, SSO, certificados e audit trail.

### Vantagens

- **Gravação de sessão**: replay completo de cada sessão SSH
- **SSO integrado**: GitHub, OIDC, SAML
- **Certificados**: sem gestão de chaves SSH
- **Auditoria completa**: quem se ligou, quando, de onde, o que foi feito

### Limitações

- **Complexo de implementar**: auth server, proxy, node agent, base de dados, certificados
- **Concebido para humanos**: sessões interativas, sem protocolo machine-to-machine
- **Sobredimensionado** para contas de serviço: um runner CI não precisa de gravação de sessão nem de SSO
- **Licença dupla**: a versão comunitária (AGPL-3.0) tem limitações funcionais

### Quando escolher Teleport

- Gestão de acesso de **pessoas** a um parque de servidores
- Necessidade de gravação de sessão e de SSO
- Infraestrutura com meios para implementar e manter um cluster

## HashiCorp Boundary

[Boundary](https://www.boundaryproject.io/) é um proxy de acesso que abstrai os detalhes de conexão e integra fontes de identidade externas.

### Vantagens

- **Abstração de infraestrutura**: os utilizadores ligam-se a alvos lógicos, não a IPs
- **Integração IAM**: Active Directory, OIDC, LDAP
- **Injeção de credenciais**: os segredos são injetados dinamicamente, nunca partilhados

### Limitações

- **Complexo**: controller, workers, base de dados, integração IAM
- **Orientado para utilizadores humanos**: não concebido para scripts automatizados
- **Licença BSL 1.1**: restrições comerciais na edição comunitária
- **Sem controlo ao nível do comando**: Boundary controla o acesso a um host, não a um comando específico

### Quando escolher Boundary

- Grande parque de servidores com gestão de identidade centralizada
- Necessidade de abstração de infraestrutura (os utilizadores não conhecem os IPs)
- Equipa com experiência HashiCorp (Vault, Terraform, etc.)

## `sudo` sozinho

`sudo` controla a elevação de privilégios para os comandos de sistema. Frequentemente utilizado sozinho para restringir as ações de uma conta de serviço.

### Vantagens

- **Nativo**: presente em todos os sistemas Linux
- **Granular**: regras finas por utilizador, comando e argumentos

### Limitações

- **Não controla a entrada SSH**: qualquer comando pode ser **pedido** via SSH, mesmo que `sudo` bloqueie a elevação
- **Sem protocolo**: sem resposta estruturada, sem logging JSON integrado
- **Configuração complexa**: as regras sudoers tornam-se difíceis de manter com muitos comandos

### Quando escolher `sudo` sozinho

- Ambiente simples onde o risco é baixo
- A entrada SSH já é controlada por outro mecanismo (bastião, VPN)

## Quando escolher SSH-Frontière

SSH-Frontière é concebido para um **caso de uso preciso**: controlar o que as contas de serviço (não os humanos) podem fazer via SSH.

Escolha SSH-Frontière se:

- As suas conexões SSH são **scripts automatizados** (CI/CD, agentes IA, cron)
- Precisa de **granularidade**: domínios, ações, argumentos, níveis de acesso
- Quer **logging JSON estruturado** para auditoria e observabilidade
- Quer uma **implementação simples**: um binário, um ficheiro TOML
- Precisa de **compatibilidade LLM**: respostas JSON, descoberta via `help`/`list`
- Não quer implementar e manter um cluster (Teleport, Boundary)

Não escolha SSH-Frontière se:

- Os seus utilizadores são **humanos** que precisam de sessões interativas ricas e completas
- Precisa de um **proxy SSH** para outros servidores
- Precisa de **SSO**
