FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    openssh-server \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir /run/sshd

# Create e2e-user
RUN useradd -m -s /usr/local/bin/ssh-frontiere e2e-user \
    && mkdir -p /home/e2e-user/.ssh /etc/ssh-frontiere /var/log/ssh-frontiere /ssh-keys \
    && chmod 700 /home/e2e-user/.ssh \
    && chmod 777 /ssh-keys \
    && chown -R e2e-user:e2e-user /home/e2e-user/.ssh /var/log/ssh-frontiere

# Restrictive sshd config
RUN echo 'PubkeyAuthentication yes\n\
PasswordAuthentication no\n\
PermitUserEnvironment no\n\
AllowUsers e2e-user\n\
MaxSessions 10\n\
PermitRootLogin no\n\
X11Forwarding no\n\
AllowAgentForwarding no\n\
AllowTcpForwarding no\n\
PermitTunnel no' > /etc/ssh/sshd_config.d/e2e.conf

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 22

HEALTHCHECK --interval=2s --retries=5 CMD ssh-keyscan localhost > /dev/null 2>&1

ENTRYPOINT ["/entrypoint.sh"]
