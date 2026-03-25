+++
title = "Déploiement"
description = "Mettre SSH-Frontière en production sur un serveur"
date = 2026-03-24
weight = 4
+++

# Déploiement

Le déploiement de SSH-Frontière se fait en 4 étapes : installer le binaire, configurer les clés SSH, modifier le login shell, et sécuriser avec sudoers.

## 1. Installer le binaire

```bash
# Copier le binaire sur le serveur
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@serveur:/usr/local/bin/

# Sur le serveur
chmod 755 /usr/local/bin/ssh-frontiere
```

## 2. Installer la configuration

```bash
# Créer le répertoire
mkdir -p /etc/ssh-frontiere

# Copier la configuration
cp config.toml /etc/ssh-frontiere/config.toml

# Sécuriser les permissions (le compte de service doit pouvoir lire la config)
chown root:forge-runner /etc/ssh-frontiere/config.toml
chmod 640 /etc/ssh-frontiere/config.toml

# Créer le répertoire de logs
mkdir -p /var/log/ssh-frontiere
chown forge-runner:forge-runner /var/log/ssh-frontiere
chmod 755 /var/log/ssh-frontiere
```

## 3. Créer le compte de service

```bash
# Créer l'utilisateur avec ssh-frontiere comme login shell
useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

Ou, si le compte existe déjà :

```bash
# Modifier le login shell
chsh -s /usr/local/bin/ssh-frontiere forge-runner
```

**Précaution** : ne fermez pas votre session en cours tant que vous n'avez pas vérifié que la connexion SSH fonctionne depuis une autre session.

## 4. Configurer les clés SSH (couche 1)

Éditez `~forge-runner/.ssh/authorized_keys` :

```
# Clé runner CI (niveau ops)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci

# Clé monitoring (niveau read seul)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring

# Clé admin (niveau admin)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-deploy
```

L'option `command=` force l'exécution de `ssh-frontiere` avec le `--level` choisi, quelle que soit la commande envoyée par le client. L'option `restrict` désactive le forwarding de port, l'agent forwarding, le PTY et les X11.

```bash
# Sécuriser les permissions
chmod 700 ~forge-runner/.ssh
chmod 600 ~forge-runner/.ssh/authorized_keys
chown -R forge-runner:forge-runner ~forge-runner/.ssh
```

## 5. Configurer sudoers (couche 3)

Créez `/etc/sudoers.d/ssh-frontiere` :

```
# SSH-Frontière : commandes autorisées pour le compte de service
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/healthcheck.sh
```

Le wildcard `*` est nécessaire pour les scripts qui reçoivent des arguments (ex: `backup-config.sh forgejo`). Les scripts sans arguments (comme `healthcheck.sh`) n'en ont pas besoin.

Validez la syntaxe :

```bash
visudo -c -f /etc/sudoers.d/ssh-frontiere
```

## 6. Vérifier

```bash
# Tester depuis un autre terminal (ne fermez pas la session en cours)

# Vérifier que les commandes disponibles s'affichent
{ echo "help"; echo "."; } | ssh forge-runner@serveur

# Tester une commande
{ echo "infra healthcheck"; echo "."; } | ssh forge-runner@serveur
```

## Défense en profondeur

Les 3 couches se complètent :

| Couche | Mécanisme | Protection |
|--------|-----------|------------|
| 1 | `command=` + `restrict` dans `authorized_keys` | Force le niveau, bloque forwarding/PTY |
| 2 | SSH-Frontière (login shell) | Valide contre la whitelist TOML |
| 3 | `sudo` dans sudoers | Restreint les commandes système |

Même si un attaquant compromet une clé SSH, il ne peut exécuter que les commandes autorisées dans la whitelist. Même s'il contourne la couche 2, les privilèges sont limités par sudoers.

## Rollback

Si quelque chose ne fonctionne pas, revenez au shell classique :

```bash
# Via la console (IPMI/KVM) ou un autre compte admin
chsh -s /bin/bash forge-runner
```

**Conseil** : sauvegardez `/etc/passwd` avant de modifier le login shell.

```bash
cp /etc/passwd /etc/passwd.bak.$(date +%Y%m%d)
```

---

**Suite** : [Premier usage](@/guides/premier-usage.md) — votre première commande SSH via SSH-Frontière.
