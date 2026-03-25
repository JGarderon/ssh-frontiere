+++
title = "Deployment"
description = "Deploy SSH-Frontière to production on a server"
date = 2026-03-24
weight = 4
+++

# Deployment

Deploying SSH-Frontière takes 4 steps: install the binary, configure SSH keys, change the login shell, and secure with sudoers.

## 1. Install the binary

```bash
# Copy the binary to the server
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@server:/usr/local/bin/

# On the server
chmod 755 /usr/local/bin/ssh-frontiere
```

## 2. Install the configuration

```bash
# Create the directory
mkdir -p /etc/ssh-frontiere

# Copy the configuration
cp config.toml /etc/ssh-frontiere/config.toml

# Secure permissions (the service account must be able to read the config)
chown root:forge-runner /etc/ssh-frontiere/config.toml
chmod 640 /etc/ssh-frontiere/config.toml

# Create the log directory
mkdir -p /var/log/ssh-frontiere
chown forge-runner:forge-runner /var/log/ssh-frontiere
chmod 755 /var/log/ssh-frontiere
```

## 3. Create the service account

```bash
# Create the user with ssh-frontiere as login shell
useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

Or, if the account already exists:

```bash
# Change the login shell
chsh -s /usr/local/bin/ssh-frontiere forge-runner
```

**Caution**: do not close your current session until you have verified that SSH connection works from another session.

## 4. Configure SSH keys (layer 1)

Edit `~forge-runner/.ssh/authorized_keys`:

```
# CI runner key (ops level)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci

# Monitoring key (read-only level)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring

# Admin key (admin level)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-deploy
```

The `command=` option forces execution of `ssh-frontiere` with the chosen `--level`, regardless of the command sent by the client. The `restrict` option disables port forwarding, agent forwarding, PTY, and X11.

```bash
# Secure permissions
chmod 700 ~forge-runner/.ssh
chmod 600 ~forge-runner/.ssh/authorized_keys
chown -R forge-runner:forge-runner ~forge-runner/.ssh
```

## 5. Configure sudoers (layer 3)

Create `/etc/sudoers.d/ssh-frontiere`:

```
# SSH-Frontière: authorized commands for the service account
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/healthcheck.sh
```

The wildcard `*` is needed for scripts that receive arguments (e.g., `backup-config.sh forgejo`). Scripts without arguments (like `healthcheck.sh`) don't need it.

Validate the syntax:

```bash
visudo -c -f /etc/sudoers.d/ssh-frontiere
```

## 6. Verify

```bash
# Test from another terminal (do not close your current session)

# Check that available commands are displayed
{ echo "help"; echo "."; } | ssh forge-runner@server

# Test a command
{ echo "infra healthcheck"; echo "."; } | ssh forge-runner@server
```

## Defense in depth

The 3 layers complement each other:

| Layer | Mechanism | Protection |
|-------|-----------|------------|
| 1 | `command=` + `restrict` in `authorized_keys` | Forces the level, blocks forwarding/PTY |
| 2 | SSH-Frontière (login shell) | Validates against the TOML whitelist |
| 3 | `sudo` in sudoers | Restricts system commands |

Even if an attacker compromises an SSH key, they can only execute commands authorized in the whitelist. Even if they bypass layer 2, privileges are limited by sudoers.

## Rollback

If something doesn't work, revert to the regular shell:

```bash
# Via the console (IPMI/KVM) or another admin account
chsh -s /bin/bash forge-runner
```

**Tip**: back up `/etc/passwd` before changing the login shell.

```bash
cp /etc/passwd /etc/passwd.bak.$(date +%Y%m%d)
```

---

**Next**: [First usage](@/guides/premier-usage.md) — your first SSH command via SSH-Frontière.
