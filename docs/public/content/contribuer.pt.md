+++
title = "Contribuir"
description = "Como contribuir para o SSH-Frontière: processo, requisitos, convenções"
date = 2026-03-24
weight = 6
+++

# Contribuir para o SSH-Frontière

As contribuições são bem-vindas, incluindo contribuições assistidas ou geradas por inteligência artificial. SSH-Frontière é ele próprio desenvolvido com agentes Claude Code.

## Antes de começar

Abra uma **issue** para discutir a alteração proposta. Isto evita trabalho desnecessário e permite validar a abordagem.

- **Bug**: descreva o comportamento observado vs esperado, a versão, o OS
- **Feature**: descreva o caso de uso e a abordagem prevista
- **Alteração arquitetural**: será necessária uma ADR (ver `docs/decisions/`)

## Processo

```
1. Issue       → discutir a alteração
2. Fork        → git checkout -b feature/minha-contribuicao
3. TDD         → RED (teste que falha) → GREEN (código mínimo) → refatorar
4. Verificar   → make lint && make test && make audit
5. Pull request → descrever, referenciar a issue, CI verde
```

## Requisitos de qualidade

SSH-Frontière é um componente de segurança. Os requisitos são rigorosos:

| Regra | Detalhe |
|-------|---------|
| Cobertura de testes | 90% mínimo para o código adicionado |
| Sem `unwrap()` | Usar `expect()` com `// INVARIANT:` ou `?` / `map_err()` |
| Sem `unsafe` | Proibido por `#[deny(unsafe_code)]` |
| 800 linhas máx. | Por ficheiro fonte |
| 60 linhas máx. | Por função |
| Formatação | `cargo fmt` obrigatório |
| Lints | `cargo clippy -- -D warnings` (pedantic) |

### Dependências

**Zero dependências não vitais.** Antes de propor uma nova dependência:

1. Verifique que a stdlib Rust não cobre a necessidade
2. Avalie com a matriz de dependências (pontuação mínima 3.5/5)
3. Documente a avaliação em `docs/searches/`

Dependências autorizadas atualmente: `serde`, `serde_json`, `toml`.

## Convenções de commit

Mensagens em **inglês**, formato `type(scope): description`:

- `feat(protocol): add TLS support`
- `fix(dispatch): handle empty arguments`
- `test(integration): add session timeout scenarios`
- `docs(references): update configuration guide`

Tipos: `feat`, `fix`, `refactor`, `test`, `docs`.

## Contribuições IA

As contribuições geradas por IA são aceites nas mesmas condições que as contribuições humanas:

- O contribuidor humano **permanece responsável** pela qualidade do código
- Mesmos requisitos de testes e lints
- Indique no PR se foi utilizado código IA (transparência)

## Segurança

### Reportar uma vulnerabilidade

**Não reporte vulnerabilidades através das issues públicas.** Contacte diretamente o mantenedor para uma divulgação responsável.

### Revisão reforçada

Os PRs que afetam estes ficheiros são sujeitos a uma revisão de segurança reforçada:

- `protocol.rs`, `crypto.rs` — autenticação
- `dispatch.rs`, `chain_parser.rs`, `chain_exec.rs` — parsing e execução dos comandos
- `config.rs` — gestão da configuração

## Boas primeiras contribuições

- Melhorar a documentação
- Adicionar testes para casos limite
- Corrigir warnings clippy
- Melhorar as mensagens de erro

## Licença

SSH-Frontière é distribuído sob [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12). Ao submeter um pull request, aceita que a sua contribuição seja distribuída nos termos desta licença.

Para os detalhes completos, consulte o ficheiro [CONTRIBUTING.md](https://github.com/nothus-forge/ssh-frontiere/blob/main/CONTRIBUTING.md) no repositório.
