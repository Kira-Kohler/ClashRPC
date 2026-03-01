# ClashRPC ⚔️

**Discord Rich Presence para Clash Royale**: muestra en tu perfil de Discord tu actividad y estado en vivo (stats del jugador + última partida), con un setup rápido.

> Proyecto: `ClashRPC`  
> Autor: **Kira Kohler**  
> Licencia: **Apache-2.0** (ver [`LICENSE`](LICENSE) y [`NOTICE`](NOTICE))

---

## 🔆 ¿Qué es y para qué sirve? 🔆

ClashRPC es una app de consola hecha en **Rust** que:

- Se conecta a **la API de Clash Royale** para leer tu perfil y battlelog.
- Se conecta al **RPC de Discord** para actualizar tu **estado** automáticamente.
- En tu estado, muestra cosas como:
  - Nombre, nivel y copas.
  - Resultado y marcador de tu última partida (victoria/derrota/empate) + modo + “hace X min”.
  - Arena actual (y su icono).
  - Botón opcional para invitar a tu clan (si pegas el link).

---

## ⚙️ Cómo funciona el código ⚙️

1. Lee la configuración desde: 📥
   - Variables de entorno (y/o archivo `.env`).
   - `config.json` (si existe).
2. Valida que `DISCORD_CLIENT_ID` exista y que el entorno (`rust_env`) sea válido.
3. Obtiene tu **API Key de Clash Royale**: 🔐
   - En **modo release**: te la pide por consola y la guarda en `config.json`.
   - En **modo no-release**: la lee desde `CLASH_ROYALE_API_KEY` o `config.json`.
4. Te pide tu **player tag** (si no está guardado) y lo valida con la API. 🏷️
5. Detecta tu clan (si tienes), y si quieres añade un botón de “Unirse al clan”. 👥
6. Entra en un loop: 🔁
   - Cada X ms refresca player + battlelog.
   - Construye el “Activity” de Discord y lo setea con Discord RPC.
   - Si Discord está cerrado, reintenta hasta reconectar.

---

## Requisitos ✅

- Discord abierto (la app usa **Discord RPC**). 💭
- Una cuenta en el **[portal oficial de Clash Royale](https://developer.clashroyale.com/)** para generar una **API Key**. 🔑

---

## Instalación y uso 🚀

### Opción A) Usar el `.exe` de **GitHub Releases** (sin tocar el código) 📦

1. [⬇️ Descarga la última versión](../../releases/latest)
2. Extrae todo en una carpeta (importante: que `ClashRPC.exe` pueda crear `config.json` al lado).
3. Abre Discord.
4. Ejecuta `ClashRPC.exe`.
   - La primera vez te pedirá tu **API Key** (por consola) y la guardará en `config.json`, (deberás pegarla y dar enter aunque parezca que no has escrito nada, se ocuta por seguridad). 🔐
   - También te pedirá tu **player tag** si no está guardado, y el link de invitación del clan (opcional). 🏷️

> Nota: En la versión de **Releases**, la configuración necesaria ya viene integrada en el `.exe`, así que **no hace falta** crear el `.env`.

### Opción B) Desarrollo (solo si vas a modificar el código) 🧑‍💻

1. Asegúrate de tener Rust instalado.
2. Copia el `.env` de ejemplo:

```powershell
# Windows (PowerShell)
Copy-Item .env.example .env
```

3. Rellena el `.env` (ver sección [`.env`](#env)).
4. Ejecuta: ▶️

```bash
cargo run
```

<details>
<summary>Compilar desde el código (opcional)</summary>

```bash
cargo build --release
```

El `.exe` se genera en `target/release/` (en Windows: `ClashRPC.exe`).

</details>

---
<a id="env"></a>
## El `.env` (solo si compilas/ejecutas desde el código fuente) 🧾

> Si estás usando el **`.exe` de GitHub Releases**, **no necesitas** `.env`.
> El programa te pedirá la **API Key** y el **player tag** la primera vez y lo guardará en `config.json`.

Copia/pega como plantilla (para desarrollo):

```dotenv
# Entorno (recomendado en desarrollo)
rust_env="development"

# Obligatorio si compilas/ejecutas desde el código:
# Application ID de la app de Clash Royale en Discord (se recomienda no cambiarlo)
DISCORD_CLIENT_ID=1112858099854876742

# Obligatorio si compilas/ejecutas desde el código:
# Token del Clash Royale API (JWT)
CLASH_ROYALE_API_KEY="TU_TOKEN_AQUI"

# Opcional: si lo pones, sale como valor por defecto al pedirte el TAG
CLASH_ROYALE_PLAYER_TAG="#ABC123"

# Base URL para los iconos de la arena, si editas esto, no funcionará.
ARENA_ASSET_BASE_URL="https://www.kirakohler.es/resources/ClashRPC"

# Debug opcional (0/1, true/false, yes/no)
DEBUG_ARENA=0
DEBUG_ARENA_FETCH=0

# Se recomiendan valores por defecto.
PLAYER_POLL_MS=30000
BATTLELOG_POLL_MS=30000
RPC_TICK_MS=5000
```
---

### 📌 Variables obligatorias

- `DISCORD_CLIENT_ID` 🆔
- `CLASH_ROYALE_API_KEY` **solo si NO estás usando el modo `release`** (por ejemplo, si estás en `development` / `production` / `test`) 🔑

### ¿Dónde se guarda la config? 💾

ClashRPC guarda/lee `config.json` en este orden: 🗂️

1. Si existe en el directorio actual, usa `./config.json`
2. Si no, lo crea al lado del ejecutable

Ejemplo de `config.json`:

```json
{
  "player_tag": "ABC123",
  "clan_invite_link": "https://link.clashroyale.com/invite/clan/es?tag=XXXX&token=YYYY",
  "clan_tag": "#XXXX",
  "clan_name": "MiClan",
  "clash_royale_api_key": "TU_TOKEN"
}
```

---

## Crear la API Key de Clash Royale (con IP autorizada) 🔑

La API de Clash Royale usa una lista de **Allowed IP addresses** (whitelist),
si tu IP cambia, tendrás que **crear una nueva key**. 🔄

Pasos:

1. Entra al portal: https://developer.clashroyale.com/
2. Crea una cuenta y ve a **My Account** → **My Keys** → **Create New Key**
3. Pon un **nombre** y **descripción**.
4. En **Allowed IP addresses**, añade tu IP pública (IPv4).
5. Crea la key y copia el **Token** (JWT).

### ¿Dónde miro mi IP pública? 🌐

- En el navegador, abre: https://api.ipify.org
  (te devuelve tu IP en texto plano)

---

## Tipos de entorno: development / production / test / release 🧪

ClashRPC valida `rust_env` y solo acepta:

- `development` 🧑‍💻
- `production` 🏁
- `test` 🧪
- `release` 📦

### Qué cambia según el entorno / modo 🔀

| Entorno | ¿Carga `.env`? | ¿De dónde sale la API key? | Notas |
|:--|:---:|:--|:--|
| development / production / test | Sí | `CLASH_ROYALE_API_KEY` (env) o `config.json` | Ideal para desarrollo y pruebas |
| release (modo release) | Depende (ver nota) | **Se pide por consola** y se guarda en `config.json` | Pensado para compartir con terceros |

**Nota importante:** “Modo release” suele aplicar al `.exe` publicado en **Releases** (y también cuando compilas con `cargo build --release`, o si fuerzas `rust_env=release`). En ese modo, **se ignora `CLASH_ROYALE_API_KEY` del entorno** y se usa el flujo interactivo.

---

## Troubleshooting 🛠️

- **No conecta a Discord RPC** 🔌
  - Asegúrate de tener Discord abierto.

- **401/403 en la API** ⛔
  - API key mal copiada o sin la IP autorizada.
  - Tu IP cambió: entra al portal y crea una key nueva.

- **No se ve el icono de la arena en Discord** 🏟️
  - Activa `DEBUG_ARENA_FETCH=1` para ver qué URL está intentando cargar. 🐛
  - Si Clash Royale añadió una arena nueva o cambió IDs, puede que **el repositorio de iconos que hosteo yo** aún no tenga el archivo correspondiente.
    - Base de iconos: `https://www.kirakohler.es/resources/ClashRPC/arenaXX.png` 🖼️
  - Soluciones rápidas:
    - 📌 Mientras tanto, puedes forzar un icono fijo con `DISCORD_SMALL_IMAGE="player"` dentro del `.env` para no depender del icono de arena hasta que actualice la customAPI.

---

## Créditos 🏷️

- **[Kira Kohler](https://kirakohler.es)** — creador y mantenedor del proyecto. 👑

---

## Licencia (Apache 2.0) 📜

Este proyecto está licenciado bajo **Apache License 2.0**. ✅

- Puedes usar, modificar y redistribuir el código. 🔓
- Debes conservar el aviso de copyright, créditos actuales y una copia de la licencia. 📌
- Si redistribuyes, también debes mantener el contenido del archivo `NOTICE` (si aplica). 📎

Lee los archivos `LICENSE` y `NOTICE` para obtener más información. 📚
