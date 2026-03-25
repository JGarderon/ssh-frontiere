+++
title = "Contribuir"
description = "Como contribuir a SSH-Frontière: proceso, requisitos, convenciones"
date = 2026-03-24
weight = 6
+++

# Contribuir a SSH-Frontière

Las contribuciones son bienvenidas, incluidas las contribuciones asistidas o generadas por inteligencia artificial. SSH-Frontière se desarrolla con agentes Claude Code.

## Antes de comenzar

Abra un **issue** para discutir el cambio propuesto. Esto evita trabajo innecesario y permite validar el enfoque.

- **Bug**: describa el comportamiento observado vs el esperado, la version, el sistema operativo
- **Feature**: describa el caso de uso y el enfoque previsto
- **Cambio arquitectonico**: sera necesaria una ADR (ver `docs/decisions/`)

## Proceso

```
1. Issue       -> discutir el cambio
2. Fork        -> git checkout -b feature/mi-contribucion
3. TDD         -> RED (test que falla) -> GREEN (codigo minimo) -> refactorizar
4. Verificar   -> make lint && make test && make audit
5. Pull request -> describir, referenciar el issue, CI verde
```

## Requisitos de calidad

SSH-Frontière es un componente de seguridad. Los requisitos son estrictos:

| Regla | Detalle |
|-------|---------|
| Cobertura de tests | 90% minimo para el codigo anadido |
| Sin `unwrap()` | Usar `expect()` con `// INVARIANT:` o `?` / `map_err()` |
| Sin `unsafe` | Prohibido por `#[deny(unsafe_code)]` |
| 800 lineas max | Por archivo fuente |
| 60 lineas max | Por funcion |
| Formateo | `cargo fmt` obligatorio |
| Lints | `cargo clippy -- -D warnings` (pedantic) |

### Dependencias

**Cero dependencias no vitales.** Antes de proponer una nueva dependencia:

1. Verifique que la stdlib de Rust no cubre la necesidad
2. Evalue con la matriz de dependencias (puntuacion minima 3.5/5)
3. Documente la evaluacion en `docs/searches/`

Dependencias autorizadas actualmente: `serde`, `serde_json`, `toml`.

## Convenciones de commit

Mensajes en **ingles**, formato `type(scope): description`:

- `feat(protocol): add TLS support`
- `fix(dispatch): handle empty arguments`
- `test(integration): add session timeout scenarios`
- `docs(references): update configuration guide`

Tipos: `feat`, `fix`, `refactor`, `test`, `docs`.

## Contribuciones IA

Las contribuciones generadas por IA se aceptan en las mismas condiciones que las contribuciones humanas:

- El contribuyente humano **sigue siendo responsable** de la calidad del codigo
- Mismos requisitos de tests y lints
- Indique en la PR si se ha utilizado codigo IA (transparencia)

## Seguridad

### Reportar una vulnerabilidad

**No reporte vulnerabilidades a traves de los issues publicos.** Contacte directamente al mantenedor para una divulgacion responsable.

### Revision reforzada

Las PR que afectan a estos archivos se someten a una revision de seguridad reforzada:

- `protocol.rs`, `crypto.rs` — autenticacion
- `dispatch.rs`, `chain_parser.rs`, `chain_exec.rs` — parsing y ejecucion de comandos
- `config.rs` — gestion de la configuracion

## Buenas primeras contribuciones

- Mejorar la documentacion
- Anadir tests para casos limite
- Corregir warnings de clippy
- Mejorar los mensajes de error

## Licencia

SSH-Frontière se distribuye bajo [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12). Al enviar una pull request, acepta que su contribucion se distribuya bajo los terminos de esta licencia.

Para los detalles completos, consulte el archivo [CONTRIBUTING.md](https://github.com/nothus-forge/ssh-frontiere/blob/main/CONTRIBUTING.md) en el repositorio.
