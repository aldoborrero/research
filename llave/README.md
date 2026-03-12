# llave

Alternative implementation of **Llave** — Spain's digital identity system for interacting with public administration services. CLI and Flutter (mobile/desktop) clients sharing a single Rust core.

## Features

- **Device activation**: Register your device with Llave
- **PIN requests**: Get a temporary Llave PIN for accessing government services
- **NIF checking**: Verify if a NIF/NIE is registered in Llave
- **DNI/NIE auth**: Weak authentication using ID card data
- **QR authentication**: Authenticate by scanning QR codes
- **Account data**: View your Llave account information
- **Operations history**: Browse past authentication operations
- **Structured output**: JSON by default, `--plain` for human-readable text
- **Secure storage**: Credentials stored via system keyring (fallback to XDG data dir)
- **Flutter apps**: iOS, Android, and desktop powered by the same Rust core via FFI

## Project Structure

```
crates/
├── llave-core/       # Shared Rust library: API client, auth flows, config, crypto, session
├── llave-core-ffi/   # FFI bridge layer for Flutter (flutter_rust_bridge)
└── llave-cli/        # CLI binary (`llave`)
flutter/              # Flutter app (iOS, Android, desktop)
```

## Installation

### With Nix (recommended)

```bash
nix run github:aldoborrero/research#llave-cli
```

### Development shell

```bash
nix develop
cargo build --release
```

### From source

```bash
cargo install --path crates/llave-cli
```

## Usage

### Activate your device

```bash
llave activate --nif 12345678Z
```

### Request a Llave PIN

```bash
llave pin
```

### Check NIF registration

```bash
llave check --nif 12345678Z
```

### DNI/NIE weak authentication

```bash
llave dni-auth --nif 12345678Z --fecha 01-01-2030 --soporte ABC123456
```

### View session status

```bash
llave status
```

### View account data

```bash
llave my-data
```

### View operations history

```bash
llave history
```

### Llave Móvil (push notification replacement)

Since the CLI cannot receive Firebase push notifications, it polls the server instead.

**Check for pending requests (single poll):**

```bash
llave pending
```

**Listen continuously for authentication requests:**

```bash
llave listen                          # default: 5s interval, 60 attempts
llave listen --interval 3 --max-attempts 120  # custom polling
```

**Confirm a pending authentication request:**

```bash
llave confirm --token <TOKEN> --idp-code <IDP_CODE>
```

**Reject a pending authentication request:**

```bash
llave reject --token <TOKEN> --idp-code <IDP_CODE>
```

### QR authentication

```bash
llave qr --value "qr-code-content"
```

### Log out

```bash
llave logout
```

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

- **Config**: `$XDG_CONFIG_HOME/llave-cli/config.json`
- **Session**: Stored in system keyring, or `$XDG_DATA_HOME/llave-cli/session.json` as fallback

## Architecture

Based on reverse engineering of the official `es.aeat.pin24h` Android app (v6.2.5). See [REVERSE_ENGINEERING.md](REVERSE_ENGINEERING.md) for the full technical analysis.

### API Base URLs

- `https://www2.agenciatributaria.gob.es/` — Primary API
- `https://www12.agenciatributaria.gob.es/` — Llave Móvil, SMS validation
- `https://pasarela.clave.gob.es/` — SAML identity gateway

## License

MIT
