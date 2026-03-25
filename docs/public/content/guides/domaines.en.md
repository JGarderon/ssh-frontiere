+++
title = "Domains and actions"
description = "Configure domains and actions in SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Configuring domains and actions

A **domain** is a functional scope (an application, a service, a category of operations). Each domain contains **actions**: the authorized commands.

## Add a deployment domain

```toml
[domains.myapp]
description = "Main web application"

[domains.myapp.actions.deploy]
description = "Deploy a version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy-myapp.sh {tag}"

[domains.myapp.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.myapp.actions.status]
description = "Check the service status"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-myapp.sh"

[domains.myapp.actions.restart]
description = "Restart the service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-myapp.sh"
```

Usage:

```bash
# Deploy the stable version
{ echo "myapp deploy version=stable"; echo "."; } | ssh ops@server

# Check status
{ echo "myapp status"; echo "."; } | ssh monitoring@server

# Restart
{ echo "myapp restart"; echo "."; } | ssh ops@server
```

## Add a backup domain

```toml
[domains.backup]
description = "Automated backups"

[domains.backup.actions.full]
description = "Full backup"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"

[domains.backup.actions.config-only]
description = "Configuration backup"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
```

## Add a notification domain

```toml
[domains.notify]
description = "Notifications"

[domains.notify.actions.slack]
description = "Send a Slack notification"
level = "ops"
timeout = 30
execute = "/usr/local/bin/notify-slack.sh {channel} {message}"

[domains.notify.actions.slack.args]
channel = { type = "enum", values = ["general", "ops", "alerts"], default = "ops" }
message = { free = true }
```

The `message` argument is declared with `free = true`: it accepts any text value.

```bash
{ echo 'notify slack channel=ops message="Deployment complete"'; echo "."; } | ssh ops@server
```

## Add a maintenance domain

```toml
[domains.infra]
description = "Server infrastructure"

[domains.infra.actions.healthcheck]
description = "Service health check"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.disk-usage]
description = "Disk space"
level = "read"
timeout = 10
execute = "/usr/bin/df -h"

[domains.infra.actions.logs]
description = "Recent system logs"
level = "ops"
timeout = 30
execute = "sudo /usr/bin/journalctl -n 100 --no-pager"
```

## Checklist after adding an action

1. Check TOML syntax (an error = fail-fast, code 129)
2. Create the execution script if needed
3. Add to sudoers if the command uses `sudo`
4. Test with `ssh user@server` from another terminal
5. Check logs in `/var/log/ssh-frontiere/commands.json`

## Discovery

The `help` and `list` commands let you see available actions:

```bash
# Full list with descriptions (readable text via #>)
{ echo "help"; echo "."; } | ssh user@server

# Domain details (readable text via #>)
{ echo "help myapp"; echo "."; } | ssh user@server

# Short list in JSON (domain + action)
{ echo "list"; echo "."; } | ssh user@server
```

`help` returns readable text (prefix `#>`). `list` returns structured JSON — more suitable for automated parsing. Both only show actions accessible at the client's effective level.

---

**Next**: [Tokens and security levels](@/guides/tokens.md) — control who can do what.
