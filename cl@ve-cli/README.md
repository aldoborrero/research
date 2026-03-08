# clave-cli

CLI tool for **Cl@ve AEAT** authentication — Spain's digital identity system used for interacting with public administration services.

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

## Installation

### With Nix (recommended)

```bash
nix run github:aldoborrero/research#clave-cli
```

### Development shell

```bash
nix develop
cargo build --release
```

### From source

```bash
cargo install --path .
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

### QR authentication

```bash
clave qr --value "qr-code-content"
```

### Log out

```bash
clave logout
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
