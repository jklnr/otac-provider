# otac-provider

A lightweight, single-purpose microservice for one-time access codes (OTACs).

**What it does:** Signs a token with your arbitrary payload, stores it, gives it to you. When you come back with it, verifies it, deletes it, gives you back the payload.

## Quick Start

```bash
# Generate a key
openssl rand -base64 32 | tr '+/' '-_' | tr -d '='
```

```bash
docker run -p 8080:8080 \
  -e OTAC_PROVIDER_JWKS='{"keys":[{"kty":"oct","kid":"key-1","k":"YOUR_BASE64URL_SECRET"}]}' \
  -e OTAC_PROVIDER_STORAGE="redis://localhost:6379" \
  otac-provider
```

## API

### POST /create

Create a new one-time access code with arbitrary JSON payload.

```bash
curl -X POST http://localhost:8080/create \
  -H "Content-Type: application/json" \
  -d '{"payload": {"username": "jay", "action": "password-reset"}}'
```

Response:
```json
{"token": "eyJhbGciOiJIUzI1NiIsImtpZCI6ImtleS0xIn0..."}
```

The `payload` can be any valid JSON: a string, number, object, array, or boolean.

```bash
# Simple string payload
curl -X POST http://localhost:8080/create \
  -d '{"payload": "user@example.com"}'

# Complex object payload
curl -X POST http://localhost:8080/create \
  -d '{"payload": {"userId": 123, "email": "user@example.com", "roles": ["admin"]}}'
```

### POST /consume

Consume a one-time access code. **Only works once.**

```bash
curl -X POST http://localhost:8080/consume \
  -H "Content-Type: application/json" \
  -d '{"token": "eyJhbGciOiJIUzI1NiIsImtpZCI6ImtleS0xIn0..."}'
```

Response (first call):
```json
{"payload": {"username": "jay", "action": "password-reset"}}
```

Response (second call):
```json
{"error": "Token not found or already consumed", "code": "TOKEN_NOT_FOUND"}
```

### GET /health

Health check endpoint.

```json
{"status": "ok"}
```

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `OTAC_PROVIDER_JWKS` | ✅ | - | JWK Set JSON containing signing keys |
| `OTAC_PROVIDER_STORAGE` | ❌ | `memory://` | keyv connection string |
| `OTAC_PROVIDER_TTL` | ❌ | `3600` | Token TTL in seconds |
| `RUST_LOG` | ❌ | `otac-provider=info` | Log level |

## Generating Keys

This project uses [JWK (JSON Web Keys)](https://datatracker.ietf.org/doc/html/rfc7517) for signing. Here's how to generate a secure key:

### Using OpenSSL (recommended)

```bash
# Generate 256 bits of randomness, base64url encode it
openssl rand -base64 32 | tr '+/' '-_' | tr -d '='
```

Example output: `dGhpcyBpcyBhIDMyIGJ5dGUgc2VjcmV0IGtleQ`

### Complete JWKS example

```json
{
  "keys": [
    {
      "kty": "oct",
      "kid": "key-2024-01",
      "k": "dGhpcyBpcyBhIDMyIGJ5dGUgc2VjcmV0IGtleQ",
      "alg": "HS256"
    }
  ]
}
```

**Fields:**
- `kty`: Key type. Must be `"oct"` for symmetric keys.
- `kid`: Key ID. Used to identify which key signed a token. Use descriptive IDs like `key-2024-01`.
- `k`: The secret key material, base64url-encoded (no padding).
- `alg`: Algorithm. Defaults to `HS256` if omitted.

## Key Rotation

To rotate keys without invalidating existing tokens:

1. **Add the new key first** in your JWKS (new key at position 0):

```json
{
  "keys": [
    {"kty": "oct", "kid": "key-2024-02", "k": "NEW_KEY_HERE"},
    {"kty": "oct", "kid": "key-2024-01", "k": "OLD_KEY_HERE"}
  ]
}
```

2. **Deploy the update.** New tokens will be signed with `key-2024-02`, but tokens signed with `key-2024-01` will still validate.

3. **After TTL expires** (e.g., 1 hour), remove the old key:

```json
{
  "keys": [
    {"kty": "oct", "kid": "key-2024-02", "k": "NEW_KEY_HERE"}
  ]
}
```

## Storage Backends

This project uses [keyv](https://crates.io/crates/keyv) for pluggable storage. Supported backends include:

* mongodb
* postgres
* redis
* mysql
* sqlite

### Redis Example

```bash
docker run -p 8080:8080 \
  -e OTAC_PROVIDER_JWKS='{"keys":[{"kty":"oct","kid":"k1","k":"YOUR_SECRET"}]}' \
  -e OTAC_PROVIDER_STORAGE="redis://redis:6379" \
  otac-provider
```

## Use Cases

- **Password reset flows**: Store `{"username": "jay"}` in the payload, consume it when they set their new password.
- **Email verification**: Store user info when they register, consume when they click the link.
- **Magic links**: Passwordless login via email.
- **Invitation links**: One-time invite URLs that can't be reused.

## Security Considerations

- **Payload is visible in the JWT**: It's base64-encoded, not encrypted. Don't put sensitive data in the payload.
- **Single-use is enforced server-side**: The JWT signature doesn't prevent replay; the storage backend does.
- **Use strong secrets**: At least 256 bits (32 bytes) of cryptographically random data.
- **Use TLS in production**: Both for the API and Redis connections.

## Building from Source

```bash
cargo build --release
```

### Docker

```bash
docker build -t otac-provider .
```

The resulting image is ~15MB (Alpine-based with statically linked binary).

## License

MIT

## AI Disclosure

The initial implementation of this project is a result of about a dozen prompts to Anthropic's Opus 4.5 LLM. It contains almost no handwritten code or documentation. It still works well though.
