+++
title = "FAQ"
description = "Preguntas frecuentes sobre SSH-Frontière"
date = 2026-03-24
weight = 5
+++

# Preguntas frecuentes

## Que es exactamente SSH-Frontière?

Un **shell de inicio de reemplazo** escrito en Rust. Se instala en lugar de `/bin/bash` en `/etc/passwd` para una cuenta de servicio. Cada conexion SSH pasa por SSH-Frontière, que valida el comando contra un archivo de configuracion TOML antes de ejecutarlo.

## Es un bastion SSH?

No. Un bastion SSH (Teleport, Boundary) es un **proxy** que reenvía las conexiones hacia otros servidores. SSH-Frontière no hace reenvio — controla lo que se ejecuta **en el servidor donde esta instalado**.

Los bastiones gestionan el acceso de personas a un parque de servidores. SSH-Frontière gestiona el acceso de **cuentas de servicio** (runners CI, agentes IA, scripts) a acciones especificas en un servidor.

## Reemplaza a `sudo`?

No, son complementarios. SSH-Frontière controla lo que el cliente SSH **puede solicitar** (capa 2). `sudo` controla los privilegios del sistema **necesarios para la ejecucion** (capa 3). Ambos se combinan para una defensa en profundidad.

## Se puede usar sin archivo TOML?

No. El archivo de configuracion es obligatorio. Es intencionado: todo es explicito, declarativo y auditable. Sin modo permisivo, sin fallback hacia un shell.

## Que pasa si la configuracion es invalida?

SSH-Frontière valida integramente la configuracion al arrancar (fail-fast). Si la configuracion es invalida, el programa se detiene con el codigo 129 y un mensaje de error explicito en el registro. Ningun comando se ejecuta. El cliente SSH, por su parte, **nunca** ve el detalle del error — solo que el servicio no esta disponible. La informacion de diagnostico permanece en el lado del servidor.

Puede probar la configuracion sin riesgo:

```bash
ssh-frontiere --check-config --config /etc/ssh-frontiere/config.toml
```

## Como diagnosticar un problema?

Hay varias herramientas disponibles:

1. **Validacion de config**: `ssh-frontiere --check-config` verifica la sintaxis y la coherencia
2. **Comando `help`**: muestra las acciones accesibles al nivel efectivo del cliente
3. **Comando `list`**: version corta (dominio + accion)
4. **Logs JSON**: cada comando (ejecutado o rechazado) se registra con timestamp, comando, argumentos, nivel, resultado
5. **Codigo de salida**: 0 = exito, 128 = rechazado, 129 = error de config, 130 = timeout, 131 = nivel insuficiente, 132 = error de protocolo, 133 = body stdin cerrado prematuramente

## Los agentes IA pueden usarlo?

Si, es un caso de uso de primera clase. Los comandos `help` y `list` devuelven JSON estructurado, directamente analizable por un agente. El protocolo de cabeceras (prefijos `+`, `#`, `$`, `>`) esta disenado para ser legible por maquinas sin perturbar la lectura humana.

Consulte la [guia de agentes IA](@/guides/agents-ia.md) para la configuracion detallada.

## Cuales son las dependencias en el codigo fuente?

3 dependencias directas:

| Crate | Uso |
|-------|-----|
| `serde` + `serde_json` | Serializacion JSON (logs, respuestas) |
| `toml` | Carga de la configuracion |

Sin runtime async, sin Tokio, sin framework web. El binario estatico ocupa ~1 Mo.

## Por que Rust y no Go/Python?

1. **Seguridad de memoria**: sin buffer overflow, sin use-after-free — critico para un componente de seguridad
2. **Binario estatico**: se compila con musl, sin dependencias del sistema
3. **Rendimiento**: arranque en milisegundos, sin runtime
4. **Sin `unsafe`**: prohibido por los lints de Cargo (`unsafe_code = "deny"`)

## Por que TOML y no YAML o JSON?

- **TOML**: legible, tipado, con comentarios, estandar en Rust, sin indentacion significativa
- **YAML**: la indentacion significativa es fuente de errores, tipos implicitos peligrosos (`on`/`off` -> booleano)
- **JSON**: sin comentarios, verboso, no disenado para configuracion humana

La eleccion esta documentada en la ADR 0001.

## Como funciona la autenticacion por token?

Dos modos:

1. **Modo simple** (`challenge_nonce = false`): el cliente calcula `SHA-256(secret)` y lo envia como proof
2. **Modo nonce** (`challenge_nonce = true`): el servidor envia un nonce, el cliente calcula `SHA-256(XOR_encrypt(secret || nonce, secret))`

El modo nonce protege contra la reutilizacion: cada proof es unico gracias al nonce.

## Se pueden usar varias claves SSH?

Si. Cada clave en `authorized_keys` tiene su propio `--level`. Varias claves pueden coexistir con niveles diferentes:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin
```

## Cual es el formato de las respuestas?

La salida estandar y de error se envian en streaming (prefijos `>>` y `>>!`), y luego una respuesta JSON final en una sola linea (prefijo `>>>`):

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` en el JSON final: la salida fue enviada en streaming
- `status_code` = 0: exito (codigo de salida del proceso hijo en passthrough)

## Como actualizar SSH-Frontière?

1. Compilar la nueva version (`make release`)
2. Copiar el binario al servidor (`scp`)
3. Verificar (`ssh user@servidor` + `help`)

Sin migracion de datos, sin esquema de base de datos. El archivo TOML se puede versionar con git.

## Como contribuir?

Consulte la [guia de contribucion](@/contribuer.md). En resumen: abrir un issue, fork, TDD, pull request, CI verde. Las contribuciones generadas por IA son aceptadas.

## Donde encontrar el codigo fuente?

El codigo fuente esta disponible en el [repositorio GitHub](https://github.com/nothus-forge/ssh-frontiere). Licencia [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
