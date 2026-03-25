+++
title = "First usage"
description = "Install SSH-Frontière, configure a first domain, and test"
date = 2026-03-24
weight = 1
+++

# First usage

This guide walks you from installation to your first SSH command via SSH-Frontière.

## 1. Prepare a minimal configuration

Create a minimal `config.toml` file:

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 60

[domains.test]
description = "Test domain"

[domains.test.actions.hello]
description = "Test command that displays a message"
level = "read"
timeout = 10
execute = "/usr/bin/echo hello from ssh-frontiere"
```

This configuration defines a single `test` domain with a `hello` action accessible at the `read` level.

## 2. Install and configure

You first need the `ssh-frontiere` binary. See the [compilation guide](@/installation/compilation.md) or download a pre-compiled binary from the [releases page](https://github.com/nothus-forge/ssh-frontiere/releases).

```bash
# Copy the binary
sudo cp ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere

# Install the configuration
sudo mkdir -p /etc/ssh-frontiere
sudo cp config.toml /etc/ssh-frontiere/config.toml
sudo chmod 640 /etc/ssh-frontiere/config.toml

# Create the log directory
sudo mkdir -p /var/log/ssh-frontiere

# Create the service account
sudo useradd -m -s /usr/local/bin/ssh-frontiere test-user

# Give the account write access to logs
sudo chown test-user:test-user /var/log/ssh-frontiere
```

## 3. Configure the SSH key

On your client machine:

```bash
# Generate a key
ssh-keygen -t ed25519 -C "test-key" -f ~/.ssh/test-frontiere
```

On the server, add the public key to `~test-user/.ssh/authorized_keys`:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... test-key
```

```bash
# Secure permissions
sudo chmod 700 ~test-user/.ssh
sudo chmod 600 ~test-user/.ssh/authorized_keys
sudo chown -R test-user:test-user ~test-user/.ssh
```

## 4. First call

```bash
# Discover available commands
{ echo "help"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@server
```

Expected response (the server sends the banner first, then the response):

```
#> ssh-frontiere 0.1.0
+> capabilities session, help, body
#> type "help" for available commands
#> ...
>>> {"command":"help","status_code":0,"status_message":"ok","stdout":null,"stderr":null}
```

The `#>` lines contain readable help text. The `help` command displays the list of domains and actions accessible at the `read` level.

## 5. Execute a command

```bash
{ echo "test hello"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@server
```

Expected response:

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

The program output (`hello from ssh-frontiere`) is sent in streaming via `>>`, then the final JSON response via `>>>`. The `stdout` and `stderr` fields are `null` in the JSON because the output was sent in streaming.

## 6. Understand the flow

Here's what happened:

1. The SSH client connects with the `test-frontiere` key
2. `sshd` authenticates the key and reads `authorized_keys`
3. The `command=` option forces execution of `ssh-frontiere --level=read`
4. SSH-Frontière displays the banner (`#>`, `+>`) and waits for headers
5. The client sends the command `test hello` (plain text, no prefix) then `.` (end of block)
6. SSH-Frontière validates: domain `test`, action `hello`, level `read` <= required `read`
7. SSH-Frontière executes `/usr/bin/echo hello from ssh-frontiere`
8. The output is sent in streaming (`>>`), then the final JSON response (`>>>`)

## 7. Test a rejection

Try a command that doesn't exist:

```bash
{ echo "test nonexistent"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@server
```

Response:

```
>>> {"command":"test nonexistent","status_code":128,"status_message":"rejected: unknown action 'nonexistent' in domain 'test'","stdout":null,"stderr":null}
```

`stdout` and `stderr` are `null` because the command was not executed.

## Next step

Now that SSH-Frontière is working, you can [configure your own domains and actions](@/guides/domaines.md).
