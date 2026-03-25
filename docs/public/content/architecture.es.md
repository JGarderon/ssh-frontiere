+++
title = "Arquitectura"
description = "Diseno tecnico de SSH-Frontière: lenguaje, modulos, protocolo, dependencias"
date = 2026-03-24
weight = 3
+++

# Arquitectura y diseno

## Por que Rust

SSH-Frontière esta escrito en Rust por tres razones:

1. **Seguridad de memoria**: sin buffer overflow, sin use-after-free, sin null pointer. Para un componente de seguridad que funciona como shell de inicio, esto es critico.

2. **Binario estatico**: se compila con el target `x86_64-unknown-linux-musl` (otros targets posibles sin garantia de funcionamiento), el binario ocupa ~1 Mo y no tiene ninguna dependencia del sistema. Se copia en el servidor y listo.

3. **Rendimiento**: el programa arranca, valida, ejecuta y termina en milisegundos. Sin runtime, sin garbage collector, sin JIT.

## Sincrono y efimero

SSH-Frontière es un programa **sincrono y one-shot**. Sin daemon, sin async, sin Tokio.

El ciclo de vida es simple:
1. `sshd` autentica la conexion SSH por clave
2. `sshd` hace fork y ejecuta `ssh-frontiere` como shell de inicio
3. `ssh-frontiere` valida y ejecuta el comando
4. El proceso termina

Cada conexion SSH crea un nuevo proceso. Sin estado compartido entre conexiones, sin problemas de concurrencia.

## Estructura del codigo

El codigo esta organizado en modulos con responsabilidades claras:

| Modulo | Responsabilidad |
|--------|-----------------|
| `main.rs` | Punto de entrada, aplanamiento de argumentos, llamada al orquestador |
| `orchestrator.rs` | Flujo principal: banner, cabeceras, comando, respuesta, bucle de sesion |
| `config.rs` | Estructuras de configuracion TOML, validacion fail-fast |
| `protocol.rs` | Protocolo de cabeceras: parser, banner, auth, sesion, body |
| `crypto.rs` | SHA-256 (implementacion FIPS 180-4), base64, nonce, challenge-response |
| `dispatch.rs` | Parsing de comandos (comillas, `key=value`), resolucion, RBAC |
| `chain_parser.rs` | Parser de cadenas de comandos (operadores `;`, `&`, `\|`) |
| `chain_exec.rs` | Ejecucion de cadenas: secuencia estricta (`;`), permisiva (`&`), recuperacion (`\|`) |
| `discovery.rs` | Comandos `help` y `list`: descubrimiento de dominios y acciones |
| `logging.rs` | Logging JSON estructurado, enmascaramiento de argumentos sensibles |
| `output.rs` | Respuesta JSON, codigos de salida |
| `lib.rs` | Exposicion de `crypto` para el binario proof y helpers de fuzz |

Cada modulo tiene su archivo de tests (`*_tests.rs`) en el mismo directorio.

Un binario auxiliar `proof` (`src/bin/proof.rs`) permite calcular los proofs de autenticacion para los tests E2E y la integracion con clientes.

## Protocolo de cabeceras

SSH-Frontière utiliza un protocolo de texto sobre stdin/stdout. Los prefijos difieren segun la direccion:

**Cliente hacia servidor (stdin):**

| Prefijo | Funcion |
|---------|---------|
| `+ ` | **Configura**: directivas (`auth`, `session`, `body`) |
| `# ` | **Comenta**: ignorados por el servidor |
| *(texto plano)* | **Comando**: `dominio accion [argumentos]` |
| `.` *(solo en una linea)* | **Fin de bloque**: termina un bloque de comando |

**Servidor hacia cliente (stdout):**

| Prefijo | Funcion |
|---------|---------|
| `#> ` | **Comenta**: banner, mensajes informativos |
| `+> ` | **Configura**: capabilities, challenge nonce |
| `>>> ` | **Responde**: respuesta JSON final |
| `>> ` | **Stdout**: salida estandar en streaming (ADR 0011) |
| `>>! ` | **Stderr**: salida de error en streaming |

### Flujo de conexion

```
CLIENTE                                 SERVIDOR
  |                                        |
  |  <-- banner + capabilities ----------  |   #> ssh-frontiere 0.1.0
  |                                        |   +> capabilities rbac, session, help, body
  |                                        |   +> challenge nonce=a1b2c3...
  |                                        |   #> type "help" for available commands
  |                                        |
  |  --- +auth (opcional) -------------->  |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (opcional) ----------->  |   + session keepalive
  |                                        |
  |  --- comando (texto plano) --------->  |   forgejo backup-config
  |  --- fin de bloque ----------------->  |   .
  |  <-- streaming stdout ---------------  |   >> Backup completed
  |  <-- respuesta JSON final -----------  |   >>> {"status_code":0,"status_message":"executed",...}
  |                                        |
  |  (si session keepalive)                |
  |  --- comando 2 --------------------->  |   infra healthcheck
  |  --- fin de bloque ----------------->  |   .
  |  <-- respuesta JSON 2 ---------------  |   >>> {"status_code":0,...}
  |  --- fin de sesion (bloque vacio) -->  |   .
  |  <-- session closed ------------------  |   #> session closed
```

### Respuesta JSON

Cada comando produce una respuesta JSON final en una sola linea, prefijada por `>>>`. La salida estandar y de error se envian en streaming mediante `>>` y `>>!`:

```
>> Backup completed
>>> {"command":"forgejo backup-config","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` en la respuesta JSON final: la salida fue enviada en streaming via `>>` y `>>!`
- Para los comandos no ejecutados (rechazo, error de config), `stdout` y `stderr` tambien son `null`

### Protocolo body

La cabecera `+body` permite transmitir contenido multilínea al proceso hijo via stdin. Cuatro modos de delimitacion:

- `+body`: lee hasta una linea que contenga unicamente `.` (punto)
- `+body size=N`: lee exactamente N bytes
- `+body stop="DELIMITADOR"`: lee hasta una linea que contenga el delimitador
- `+body size=N stop="DELIMITADOR"`: el primer delimitador alcanzado (tamano o marcador) finaliza la lectura

## Configuracion TOML

El formato de configuracion es TOML declarativo. Eleccion documentada en la ADR 0001:

- **Por que TOML**: legible por humanos, tipado nativo, estandar en el ecosistema Rust, sin indentacion significativa (a diferencia de YAML), mas expresivo que JSON para configuracion.
- **Por que no YAML**: la indentacion significativa es fuente de errores, tipos implicitos peligrosos (`on`/`off` -> booleano), especificacion compleja.
- **Por que no JSON**: sin comentarios, verboso, no disenado para configuracion humana.

La configuracion se **valida al cargarse** (fail-fast): sintaxis TOML, completitud de campos, coherencia de los placeholders, al menos un dominio, al menos una accion por dominio, valores enum no vacios.

## Politica de dependencias

SSH-Frontière tiene una politica de **cero dependencias no vitales**. Cada crate externa debe estar justificada por una necesidad real.

### Dependencias actuales

3 dependencias directas, ~20 dependencias transitivas:

| Crate | Uso |
|-------|-----|
| `serde` + `serde_json` | Serializacion JSON (logging, respuestas) |
| `toml` | Carga de la configuracion TOML |

### Matriz de evaluacion

Antes de agregar una dependencia, se evalua segun 8 criterios ponderados (nota /5): licencia (eliminatorio), gobernanza (x3), comunidad (x2), frecuencia de actualizacion (x2), tamano (x3), dependencias transitivas (x3), funcionalidades (x2), no encerramiento (x1). Puntuacion minima: 3.5/5.

### Auditoria

- `cargo deny` verifica las licencias y las vulnerabilidades conocidas
- `cargo audit` busca fallos en la base RustSec
- Fuentes autorizadas: crates.io unicamente

## Como se diseno el proyecto

SSH-Frontière se desarrollo en fases sucesivas (1 a 9, con fases intermedias 2.5 y 5.5), pilotado por agentes Claude Code con una metodologia TDD sistematica:

| Fase | Contenido |
|------|-----------|
| 1 | Dispatcher funcional, config TOML, RBAC 3 niveles |
| 2 | Configuracion produccion, scripts de operaciones |
| 2.5 | SHA-256 FIPS 180-4, BTreeMap, timeout graceful |
| 3 | Protocolo de cabeceras unificado, auth challenge-response, sesiones |
| 4 | Tests E2E SSH Docker, limpieza de codigo, integracion forge |
| 5 | Tags de visibilidad, filtrado horizontal por tokens |
| 5.5 | Nonce opcional, argumentos con nombre, binario proof (incluye la fase 6, fusionada) |
| 7 | Guia de configuracion, dry-run `--check-config`, help sin prefijo |
| 8 | Tipos de error estructurados, clippy pedantic, cargo-fuzz, proptest |
| 9 | Protocolo body, argumentos libres, max_body_size, codigo de salida 133 |

El proyecto fue disenado por:
- **Julien Garderon** (BO): concepto, especificaciones funcionales, eleccion de Rust, nombre del proyecto
- **Claude supervisor** (PM/Tech Lead): analisis tecnico, arquitectura
- **Agentes Claude Code**: implementacion, tests, documentacion

Donde el humano y la maquina trabajan juntos, mejor, mas rapido, con mayor seguridad.
