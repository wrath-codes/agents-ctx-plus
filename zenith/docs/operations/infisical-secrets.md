# Infisical Secrets Bootstrap

Zenith can load configuration secrets from Infisical before constructing `ZenConfig`.

## Enablement

Secret loading is opt-in.

```bash
export ZENITH_SECRETS__BACKEND=infisical
```

Required variables:

- `ZENITH_INFISICAL__CLIENT_ID`
- `ZENITH_INFISICAL__CLIENT_SECRET`
- `ZENITH_INFISICAL__PROJECT_ID`
- `ZENITH_INFISICAL__ENVIRONMENT`

Optional variables:

- `ZENITH_INFISICAL__BASE_URL` (default: `https://app.infisical.com`)
- `ZENITH_INFISICAL__PATH` (default: `/`)

## Secret Naming

Store secrets in Infisical using exact Zenith env keys, for example:

- `ZENITH_CLERK__SECRET_KEY`
- `ZENITH_TURSO__URL`
- `ZENITH_TURSO__AUTH_TOKEN`

Only keys with the `ZENITH_` prefix are consumed.

## Precedence

When building `ZenConfig`, precedence is:

1. Process environment (`ZENITH_*`)
2. Infisical-loaded values
3. `.zenith/config.toml`
4. `~/.config/zenith/config.toml`
5. Built-in defaults

This means manually exported env vars always win over Infisical values.

## Runtime Behavior

- Local/dev: if Infisical loading fails, Zenith logs a warning and continues.
- CI (`CI=true`): if Infisical loading fails while backend is enabled, Zenith exits with an error.

## Dotenv Resolution

Before loading external secrets, CLI bootstrap attempts to load dotenv in this order:

1. `--project <path>/.env` (if provided and present)
2. nearest project root `.env` (walking up to a directory containing `.zenith`)
3. cwd `.env` fallback

This allows commands like `znt auth status` to work from outside the project root when a
project `.env` exists.
