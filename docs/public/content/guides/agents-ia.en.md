+++
title = "AI agents"
description = "Using SSH-Frontière with AI agents (Claude Code, etc.)"
date = 2026-03-24
weight = 4
+++

# Using SSH-Frontière with AI agents

SSH-Frontière was designed from the start to be compatible with AI agents (LLMs). The structured protocol, automatic discovery, and JSON responses make it an ideal entry point for agents that need to act on a server.

## Why SSH-Frontière for AI agents?

AI agents (Claude Code, Cursor, GPT, etc.) can execute commands on a server via SSH. The problem: without control, an agent can execute anything.

SSH-Frontière solves this:

- **Bound the actions**: the agent can only execute configured commands
- **Access levels**: an agent at `read` can only consult, not modify
- **Discovery**: the agent can ask `help` to learn available actions
- **Structured JSON**: responses are directly parsable by the agent

## Configuration for an AI agent

### 1. Dedicated SSH key

Generate an SSH key for the agent:

```bash
ssh-keygen -t ed25519 -C "agent-claude" -f ~/.ssh/agent-claude
```

### 2. Restricted trust level

In `authorized_keys`, give a minimal level:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent-claude
```

Start with `read` and elevate if needed via a token.

### 3. Dedicated domains

Configure specific actions for the agent:

```toml
[domains.agent]
description = "Actions for AI agents"

[domains.agent.actions.status]
description = "Service status"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-all.sh"

[domains.agent.actions.logs]
description = "Recent application logs"
level = "read"
timeout = 30
execute = "/usr/local/bin/recent-logs.sh {service}"

[domains.agent.actions.logs.args]
service = { type = "enum", values = ["web", "api", "worker", "database"] }

[domains.agent.actions.restart]
description = "Restart a service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-service.sh {service}"
tags = ["agent-ops"]

[domains.agent.actions.restart.args]
service = { type = "enum", values = ["web", "api", "worker"] }
```

### 4. Token for elevation (optional)

If the agent needs access to `ops` actions:

```toml
[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "ops"
tags = ["agent-ops"]
```

## Example with Claude Code (AutoClaude)

A Claude Code agent in an AutoClaude container can use SSH-Frontière to act on the host server:

```bash
# The agent discovers available commands (JSON via list)
{ echo "list"; echo "."; } | ssh -i /keys/agent-claude agent@server

# The agent checks service status
{ echo "agent status"; echo "."; } | ssh -i /keys/agent-claude agent@server

# The agent reads service logs
{ echo "agent logs service=api"; echo "."; } | ssh -i /keys/agent-claude agent@server
```

The output is sent in streaming (`>>`), then the final JSON response (`>>>`):

```
>> web: running
>> api: running
>> worker: stopped
>>> {"command":"agent status","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

The agent can analyze the `>>` lines (standard output streaming), detect that `worker` is stopped, and decide to act accordingly. The `>>>` response confirms the return code.

## Session mode

To avoid opening an SSH connection per command, the agent can use session mode:

```bash
{
  echo "+ auth token=agent-claude proof=..."
  echo "+ session keepalive"
  echo "agent status"
  echo "."
  echo "agent logs service=worker"
  echo "."
  echo "."   # empty block = end of session
} | ssh -i /keys/agent-claude agent@server
```

Each command is followed by `.` (end of block). A `.` without a preceding command signals the end of session. Session mode allows sending multiple commands in a single SSH connection, with a configurable global timeout (`timeout_session`).

## Best practices

1. **Principle of least privilege**: start with `read`, elevate by token only if necessary
2. **Atomic actions**: each action does one thing. The agent composes actions together
3. **Explicit names**: domain and action names are visible via `help` — make them understandable
4. **Visibility tags**: isolate agent actions with dedicated tags
5. **Output limits**: configure `max_stdout_chars` to prevent the agent from receiving excessive volumes
6. **Logs**: monitor logs to detect abnormal usage

---

**Next**: [CI/CD integration](@/guides/ci-cd.md) — automate deployments via SSH-Frontière.
