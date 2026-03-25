+++
title = "Primeira utilização"
description = "Instalar SSH-Frontière, configurar um primeiro domínio e testar"
date = 2026-03-24
weight = 1
+++

# Primeira utilização

Este guia acompanha-o desde a instalação até ao seu primeiro comando SSH via SSH-Frontière.

## 1. Preparar uma configuração mínima

Crie um ficheiro `config.toml` mínimo:

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 60

[domains.test]
description = "Domínio de teste"

[domains.test.actions.hello]
description = "Comando de teste que apresenta uma mensagem"
level = "read"
timeout = 10
execute = "/usr/bin/echo hello from ssh-frontiere"
```

Esta configuração define um único domínio `test` com uma ação `hello` acessível ao nível `read`.

## 2. Instalar e configurar

Primeiro precisa de dispor do binário `ssh-frontiere`. Consulte o [guia de compilação](@/installation/compilation.md) ou descarregue um binário pré-compilado a partir da [página de releases](https://github.com/nothus-forge/ssh-frontiere/releases).

```bash
# Copiar o binário
sudo cp ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere

# Instalar a configuração
sudo mkdir -p /etc/ssh-frontiere
sudo cp config.toml /etc/ssh-frontiere/config.toml
sudo chmod 640 /etc/ssh-frontiere/config.toml

# Criar o diretório de logs
sudo mkdir -p /var/log/ssh-frontiere

# Criar a conta de serviço
sudo useradd -m -s /usr/local/bin/ssh-frontiere test-user

# Dar à conta acesso de escrita aos logs
sudo chown test-user:test-user /var/log/ssh-frontiere
```

## 3. Configurar a chave SSH

Na sua máquina cliente:

```bash
# Gerar uma chave
ssh-keygen -t ed25519 -C "test-key" -f ~/.ssh/test-frontiere
```

No servidor, adicione a chave pública em `~test-user/.ssh/authorized_keys`:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... test-key
```

```bash
# Proteger as permissões
sudo chmod 700 ~test-user/.ssh
sudo chmod 600 ~test-user/.ssh/authorized_keys
sudo chown -R test-user:test-user ~test-user/.ssh
```

## 4. Primeira chamada

```bash
# Descobrir os comandos disponíveis
{ echo "help"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@servidor
```

Resposta esperada (o servidor envia primeiro o banner, depois a resposta):

```
#> ssh-frontiere 0.1.0
+> capabilities session, help, body
#> type "help" for available commands
#> ...
>>> {"command":"help","status_code":0,"status_message":"ok","stdout":null,"stderr":null}
```

As linhas `#>` contêm o texto de ajuda legível. O comando `help` apresenta a lista dos domínios e ações acessíveis ao nível `read`.

## 5. Executar um comando

```bash
{ echo "test hello"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@servidor
```

Resposta esperada:

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

A saída do programa (`hello from ssh-frontiere`) é enviada em streaming via `>>`, depois a resposta JSON final via `>>>`. Os campos `stdout` e `stderr` são `null` no JSON porque a saída foi enviada em streaming.

## 6. Compreender o fluxo

Eis o que aconteceu:

1. O cliente SSH liga-se com a chave `test-frontiere`
2. `sshd` autentica a chave e lê o `authorized_keys`
3. A opção `command=` força a execução de `ssh-frontiere --level=read`
4. SSH-Frontière apresenta o banner (`#>`, `+>`) e aguarda os cabeçalhos
5. O cliente envia o comando `test hello` (texto simples, sem prefixo) depois `.` (fim de bloco)
6. SSH-Frontière valida: domínio `test`, ação `hello`, nível `read` <= `read` exigido
7. SSH-Frontière executa `/usr/bin/echo hello from ssh-frontiere`
8. A saída é enviada em streaming (`>>`), depois a resposta JSON final (`>>>`)

## 7. Testar uma rejeição

Tente um comando que não existe:

```bash
{ echo "test inexistente"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@servidor
```

Resposta:

```
>>> {"command":"test inexistente","status_code":128,"status_message":"rejected: unknown action 'inexistente' in domain 'test'","stdout":null,"stderr":null}
```

`stdout` e `stderr` são `null` porque o comando não foi executado.

## Próximo passo

Agora que SSH-Frontière funciona, pode [configurar os seus próprios domínios e ações](@/guides/domaines.md).
