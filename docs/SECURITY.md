# Kabootar — säkerhetsverktygslåda

Kabootar är **säkerhetsagnostisk**: språket ger vapen, applikationen bestämmer kriget.

## Filosofi

| Kabootar (språket) | Utvecklaren (applikationen) |
|--------------------|-----------------------------|
| Kryptografiska primitiver | Zero-knowledge vs moln? |
| Säker nyckelhantering i minnet | HSM för 50 000 kr eller YubiKey? |
| Enhets-API (USB, TPM, smartkort) | Central eller lokal databas? |
| Utbytbara security providers | Quorum (2 av 3) eller en admin? |

Kabootar **beslutar inte** vilken säkerhetsarkitektur du ska ha. Du får byggstenar och ett gemensamt gränssnitt.

## Kryptografiska primitiver

Alla funktioner tar **byte-arrayer** (`[1, 2, 3]`) eller `crypto_secure()`-buffertar.

| Funktion | Algoritm |
|----------|----------|
| `crypto_random(n)` | OS CSPRNG (via aktiv provider) |
| `crypto_sha3_256(data)` | SHA-3-256 |
| `crypto_sha3_512(data)` | SHA-3-512 |
| `crypto_argon2(pw, salt, m, t, p)` | Argon2id (lösenordshash) |
| `crypto_aes256_encrypt(key, nonce, plain)` | AES-256-GCM |
| `crypto_aes256_decrypt(key, nonce, cipher)` | AES-256-GCM |
| `crypto_chacha20_encrypt(key, nonce, plain)` | ChaCha20-Poly1305 |
| `crypto_chacha20_decrypt(key, nonce, cipher)` | ChaCha20-Poly1305 |
| `crypto_rsa_generate(bits)` | RSA (2048–4096) |
| `crypto_rsa_encrypt(pub, plain)` | RSA PKCS#1 v1.5 |
| `crypto_rsa_decrypt(priv, cipher)` | RSA PKCS#1 v1.5 |
| `crypto_ecc_generate()` | ECDSA P-256 |
| `crypto_ecc_sign(priv, msg)` | ECDSA P-256 |
| `crypto_ecc_verify(pub, msg, sig)` | ECDSA P-256 |

Nycklar och klartext kan lagras i `crypto_secure()` — se nedan.

## Säker nyckelhantering

```kabootar
let key = crypto_secure(crypto_random(32));
// ... använd key ...
crypto_wipe(key);   // nollställer bufferten i minnet
```

- `crypto_secure(data)` — känslig buffert (delad via Arc; wipe påverkar alla referenser)
- `crypto_wipe(buf)` — explicit nollställning
- `crypto_is_secure(val)` — kontrollera om värdet är en secure buffer

**OBS:** Kabootar kan inte garantera att OS/minnesdump inte läcker data — `crypto_wipe` minskar risken i processen.

## Pluggable Security (providers)

```kabootar
security_list_providers();          // alla backends
security_use_provider("software");  // standard CPU-krypto
security_use_provider("tpm-stub");  // TPM-lik stub (byt mot riktig drivrutin)
security_provider();                // aktiv provider
security_capabilities();            // vad aktiv provider stöder
```

| Provider | Syfte |
|----------|-------|
| `software` | Standard — alla primitiver i CPU |
| `tpm-stub` | TPM 2.0-stub (slump från simulerad TPM) |
| `yubikey-stub` | Smartkort / YubiKey-stub |
| `hsm-stub` | HSM-stub (enterprise-quorum-flöden) |

Providers delar samma gränssnitt — byt backend utan att ändra applikationslogik.

## Enhets-API

Enhetlig åtkomst till USB, TPM och smartkort utan att känna till hårdvarudetaljer:

```kabootar
device_list();                    // [{id, kind, name}, ...]
let h = device_open("tpm-0");
device_read(h, 32);
device_write(h, [1, 2, 3]);
device_close(h);
```

Stub-enheter (`usb-0`, `tpm-0`, `sc-0`) finns för utveckling. Produktionsdrivrutiner kopplas in bakom samma API.

## Modul

```kabootar
import "crypto"
let digest = sha256("password")   // wrapper → crypto_sha3_256
let key = secure(crypto_random(32))
```

## Bygg

Krypto är på som standard (`default = ["crypto"]`):

```bash
cargo build
cargo test
```

Utan krypto (t.ex. minimal WASM):

```bash
cargo build --no-default-features
```

## Exempel — applikationsval (inte Kabootars jobb)

```kabootar
// Utvecklaren väljer: lokal Argon2-hash, ingen moln-HSM
let salt = crypto_random(16);
let hash = crypto_argon2("user-password", salt, 19456, 2, 1);

// Utvecklaren väljer: TPM-baserad entropi
security_use_provider("tpm-stub");
let session_key = crypto_secure(crypto_random(32));
```

Kabootar levererar verktygen. **Du** bestämmer hotmodell, lagring och quorum.
