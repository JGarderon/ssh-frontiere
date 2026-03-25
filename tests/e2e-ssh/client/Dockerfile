FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    openssh-client \
    jq \
    coreutils \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Create e2e user for running tests
RUN useradd -m e2e \
    && mkdir -p /ssh-keys \
    && chown e2e:e2e /ssh-keys

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

USER e2e
WORKDIR /home/e2e

ENTRYPOINT ["/entrypoint.sh"]
CMD ["sleep", "infinity"]
