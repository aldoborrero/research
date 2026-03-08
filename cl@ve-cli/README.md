# clave

Alternative implementation of **Cl@ve AEAT** — Spain's digital identity system for interacting with public administration services. CLI, web, and mobile clients sharing a single Rust core.

## Features

- **Device activation**: Register your device with Cl@ve
- **PIN requests**: Get a temporary Cl@ve PIN for accessing government services
- **NIF checking**: Verify if a NIF/NIE is registered in Cl@ve
- **DNI/NIE auth**: Weak authentication using ID card data
- **QR authentication**: Authenticate by scanning QR codes
- **Account data**: View your Cl@ve account information
- **Operations history**: Browse past authentication operations
- **Structured output**: JSON by default, `--plain` for human-readable text
- **Secure storage**: Credentials stored via system keyring (fallback to XDG data dir)
- **Web interface**: Browser-based UI with real-time SSE updates for pending requests
- **Mobile apps**: Flutter (iOS + Android) powered by the same Rust core via FFI

## Project Structure

```
crates/
├── clave-core/       # Shared Rust library: API client, auth flows, config, crypto, session
├── clave-core-ffi/   # FFI bridge layer for Flutter (flutter_rust_bridge)
├── clave-cli/        # CLI binary (`clave`)
└── clave-web/        # Web server binary (`clave-web`) with embedded frontend
flutter/              # Flutter mobile app (iOS, Android, desktop)
```

## Installation

### With Nix (recommended)

```bash
nix run github:aldoborrero/research#clave-cli   # CLI
nix run github:aldoborrero/research#clave-web   # Web UI
```

### Development shell

```bash
nix develop
cargo build --release
```

### From source

```bash
cargo install --path crates/clave-cli   # CLI
cargo install --path crates/clave-web   # Web UI
```

## Usage

### Activate your device

```bash
clave activate --nif 12345678Z
```

### Request a Cl@ve PIN

```bash
clave pin
```

### Check NIF registration

```bash
clave check --nif 12345678Z
```

### DNI/NIE weak authentication

```bash
clave dni-auth --nif 12345678Z --fecha 01-01-2030 --soporte ABC123456
```

### View session status

```bash
clave status
```

### View account data

```bash
clave my-data
```

### View operations history

```bash
clave history
```

### Cl@ve Móvil (push notification replacement)

Since the CLI cannot receive Firebase push notifications, it polls the server instead.

**Check for pending requests (single poll):**

```bash
clave pending
```

**Listen continuously for authentication requests:**

```bash
clave listen                          # default: 5s interval, 60 attempts
clave listen --interval 3 --max-attempts 120  # custom polling
```

**Confirm a pending authentication request:**

```bash
clave confirm --token <TOKEN> --idp-code <IDP_CODE>
```

**Reject a pending authentication request:**

```bash
clave reject --token <TOKEN> --idp-code <IDP_CODE>
```

### QR authentication

```bash
clave qr --value "qr-code-content"
```

### Log out

```bash
clave logout
```

### Web interface

Start the web server (requires an active session from `clave activate`):

```bash
clave-web
```

Then open `http://127.0.0.1:3000` in your browser. The web UI provides:

- Real-time pending request notifications via Server-Sent Events (SSE)
- PIN requests, QR auth, history, and account data views
- Confirm/reject pending authentication requests from the browser

## Output formats

By default, all commands output **JSON**:

```json
{
  "pin": "123456",
  "time_to_live_seconds": "180",
  "nif": "12345678Z"
}
```

Use `--plain` for human-readable output:

```
pin: 123456
time_to_live_seconds: 180
nif: 12345678Z
```

Use `-v` / `--verbose` for debug logging.

## Configuration

Configuration and session data are stored following XDG conventions:

- **Config**: `$XDG_CONFIG_HOME/clave-cli/config.json`
- **Session**: Stored in system keyring, or `$XDG_DATA_HOME/clave-cli/session.json` as fallback

## Architecture

Based on reverse engineering of the official `es.aeat.pin24h` Android app (v6.2.5). See [REVERSE_ENGINEERING.md](REVERSE_ENGINEERING.md) for the full technical analysis.

### API Base URLs

- `https://www2.agenciatributaria.gob.es/` — Primary API
- `https://www12.agenciatributaria.gob.es/` — Cl@ve Móvil, SMS validation
- `https://pasarela.clave.gob.es/` — SAML identity gateway

## License

MIT
