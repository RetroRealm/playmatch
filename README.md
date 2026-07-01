<div align="center">
  <picture>
    <source srcset=".github/resources/Playmatch%20Inverted%20Color%20Transparent%20bg.svg" media="(prefers-color-scheme: dark)">
    <source srcset=".github/resources/Playmatch%20Main%20Logo%20Transparent%20bg.svg" media="(prefers-color-scheme: light)">
    <img src=".github/resources/Playmatch%20Main%20Logo%20Transparent%20bg.svg" height="180" alt="Playmatch logo">
  </picture>
  <h3>Identify and Match your ROMs at blazingly fast speed</h3>
</div>



Playmatch is an open-source service to match, identify and verify your ROMs while also providing & caching metadata for them.

The public API is available at [playmatch.retrorealm.dev](https://playmatch.retrorealm.dev/swagger-ui/)

## Features

### Supported

- [x] Supports No-Intro, Redump (Public and Private) and certain Community Dat files
- [x] Automatically daily downloads and updates dat files
- [x] Hash dat files to skip daily import if nothing changed
- [x] Support for IGDB as metadata provider
- [x] Full IGDB entity endpoints exposed through a caching proxy
- [x] Support for SteamGridDB as metadata provider
- [x] Support for ScreenScraper as metadata provider
- [x] Support for MobyGames as metadata provider
- [x] Support for LaunchBox as metadata provider (bulk metadata import, opt-in via `LAUNCHBOX_ENABLED`)
- [x] Support for OpenVGDB as metadata provider (offline SQLite hash matching, opt-in via `OPENVGDB_ENABLED`)
- [x] Support for EmuReady as metadata provider (id-mapping only, opt-in via `EMUREADY_ENABLED`)
- [x] Support for RetroAchievements as metadata provider (bulk import via API, MD5 hash matching plus name fallback, opt-in via `RETROACHIEVEMENTS_USERNAME` + `RETROACHIEVEMENTS_API_KEY`)
- [x] Support for TheGamesDB as metadata provider (catalogue seeded via migration, capped API search-on-miss with Redis-backed quota, opt-in via `TGDB_ENABLED` plus optional `TGDB_API_KEY`)
- [x] Support for Hasheous as metadata provider (hash-based matching against public signature DATs including No-Intro, Redump, TOSEC, MAMEArcade and RetroAchievements, opt-in via `HASHEOUS_ENABLED`)

### Planned

- [ ] Support for more dat files sources (TOSEC, MAME, GoodTools, etc)
- [ ] Support for more metadata providers
- [ ] Support bios and other non-game files, which you can also hash and verify this way

## Getting Started

### Prerequisites

1. Rust 1.95+ from [here](https://www.rust-lang.org/tools/install)
2. PostgreSQL 18+ from [here](https://www.postgresql.org/download/)
3. A Redis-compatible server such as [Redis](https://redis.io/download), [Valkey](https://valkey.io/), or [DragonflyDB](https://www.dragonflydb.io/)

### Development

1. Clone the repository
2. Rename .env.example to .env and fill in the required environment variables
3. Run `cargo run --package playmatch --bin playmatch` to start the server

### Testing

Run the full suite with `cargo test --workspace`. The integration tests start throwaway PostgreSQL and Redis containers through testcontainers, so a running Docker daemon is required.

## HTTP API

The API is versioned by path.

* `GET /api/v1/*` is the full v1 surface. It is permanently frozen and never
  breaks. All provider proxy routes live here.
* `GET /api/v2/*` is additive on top of v1. It carries full v1 parity: all
  provider proxy routes, the shared public routes, and the reused v1
  authenticated handlers (suggestions, users, manual match). On top of that it
  adds the v2-only endpoints: the keyset list endpoints, search, bulk, stats,
  and reverse lookups.
* `GET /api/*` is a bare alias that mirrors whichever version is configured as
  the default.

The default version is `v1`. It is resolved once at boot and stays fixed for the
process lifetime. Override it with `PLAYMATCH_DEFAULT_API_VERSION` set to `v1` or
`v2` (case-insensitive). An unset or unparseable value falls back to `v1`.

Version-scoped responses carry a `Playmatch-Api-Version` header naming the
resolved version. The same name used as a request header is only a proxy-pinning
hint, the server routes by path and not by this header.

`GET /api/versions` is unversioned, public, needs no auth, and is cached for an
hour. It returns the `default` and `latest` versions, the `request_header` name,
and a `versions` array where each entry lists its status, `is_default`, lifecycle
dates, `docs_url` and `openapi_url`.

OpenAPI documents are served per version:

* `GET /api-docs/v1/openapi.json` (servers `/api/v1`)
* `GET /api-docs/v2/openapi.json` (servers `/api/v2`)
* `GET /api-docs/openapi.json` aliases the default version

Swagger UI is at `/swagger-ui/` with three tabs: v2 (primary), v1, and the alias.

### Pagination

All v2 list endpoints use keyset (cursor) pagination.

* `limit` is clamped to `[1, 50]`. Absent or `0` falls back to `25`. Out-of-range
  values are clamped, never rejected with a 400.
* `cursor` is an opaque base64url string with no padding. Absent or empty starts
  at page 1.
* `withTotal` is honored only on platforms and signature-groups. It is ignored
  on games, companies and dat-files.

Responses are wrapped in a fixed envelope:

```json
{
  "data": [],
  "pagination": {
    "limit": 25,
    "hasNextPage": false,
    "hasPreviousPage": false,
    "nextCursor": null,
    "totalItems": null
  }
}
```

`hasNextPage` is true when there are more rows after the current page.
`hasPreviousPage` is true when a cursor was supplied, meaning the response is
not the first page. `totalItems` is only present when `withTotal=true` is
requested, and only on the bounded reference tables (platforms and
signature-groups) that support it; it is null or absent everywhere else.

Cursor handling has three outcomes. A cursor presented against a changed filter
returns `400 { "code": "cursor_filter_mismatch", "message": ..., "restart": true }`.
A malformed cursor returns `400 { "code": "malformed_cursor", ... }`. A cursor from
a stale format version silently restarts at page 1.

### Bulk endpoints

Eight v2 endpoints accept a batch in one request:

* `POST /api/v2/games/bulk`
* `POST /api/v2/platforms/bulk`
* `POST /api/v2/companies/bulk`
* `POST /api/v2/signature-groups/bulk`
* `POST /api/v2/dat-files/bulk`
* `POST /api/v2/game-files/bulk`
* `POST /api/v2/identify/bulk/ids`
* `POST /api/v2/identify/bulk/relations`

`MAX_BULK_ITEMS` is 100 per request and the JSON body is capped at 256 KiB. Each
bulk request counts as exactly one request against the per-IP rate limiter, no
matter how many items it carries. A request over the cap returns `400` with an
`X-Bulk-Max-Items: 100` header.

Partial failures do not fail the batch. The response stays `200` and reports a
per-item result (`ok`/`notFound` for get-by-id, or `ok`/`invalid`/`error` for
identify) for each entry.

### Authentication

Most v2 read endpoints are public. The v2 suggestion endpoint requires a bearer
token with the `Automation` permission, returning `401` when the token is missing
and `403` when it is insufficient. API keys are HMAC-hashed with `API_KEY_PEPPER`.
Permission tiers run `User` < `Trusted` < `Automation` < `Admin`.

### Errors

Every v2 `4xx` and `5xx` response is a machine-readable JSON envelope:

```json
{ "code": "game_not_found", "message": "game not found" }
```

`code` is a stable string a client can branch on; `message` is human-readable.
Some errors carry extra fields: a rejected pagination cursor adds `"restart": true`,
and a batch over the cap adds `"limit"` and `"received"`. The envelope is uniform
across the whole v2 surface, including body/query/path validation, auth
(`401`/`403`), not-found, the rate-limit `429`, and unmatched routes. v1 keeps its
historical plain-text error bodies unchanged.

### Identify

The identify endpoints match a ROM against the catalogue and return the game it
belongs to. Both versions expose the same two routes:

* `GET .../identify/ids` returns the matched game with its metadata provider ids.
* `GET .../identify/relations` returns the matched game together with its game
  files, metadata mappings, signature group, publisher, and company.

Use the v1 paths under `/api/v1` or the v2 paths under `/api/v2`. Pass the file
hashes and, as a fallback, the filename and size as query parameters.

The strongest supplied hash resolves the file. Hashes are tried in order: sha256,
then sha1, then md5, then crc. The crc rung matches on crc and file size together,
since a 32-bit crc alone collides at collection scale, so pass the size alongside a
crc. When no hash matches, the filename and size are tried last. A no-match outcome
is still a `200` with an empty result.

The v2 `identify/relations` response adds an `additionalMatches` array. When the
resolving hash is shared by sibling games under other signature providers, those
co-hashed games are listed there, ranked after the primary match. The array is
omitted for single-game hash results and for filename and size matches.

## MCP server

Playmatch ships a Model Context Protocol server so AI agents can identify ROMs
and browse the catalogue with the same data the public API serves. It is mounted
on the main API listener and speaks MCP over Streamable HTTP at `/mcp` (so the
endpoint is `http://<host>:<PORT>/mcp`). It shares the public API per-IP rate
limit.

It is controlled by these environment variables:

* `MCP_ENABLED` - set to `false` to disable the server. Defaults to `true`.
* `MCP_PUBLIC_URL` - the public origin of the deployment (for example
  `https://playmatch.retrorealm.dev`). Used to advertise the endpoint in the
  discovery document. Leave empty to omit the remote entry.

The server publishes an MCP Server Card at `/.well-known/mcp-server-card` (and the
same document at `/.well-known/mcp.json`). A client that already knows the host can
read the card to learn the endpoint and metadata without connecting. This is a
self-hosted card, not a registry listing, so it does not by itself make registries
aware of the server.

The MCP session manager is kept in-process, so the deployment must run as a single
instance. This is the same constraint as the in-process cron and import jobs.

The server exposes these 18 tools:

* `playmatch_identify_rom_by_hash` - identify a ROM and return its game id and
  external metadata provider ids.
* `playmatch_identify_rom_with_relations` - identify a ROM and return the game
  with its platform, company, signature group, dat file and files.
* `playmatch_get_game` - fetch a single game and its external metadata by id.
* `playmatch_get_game_with_relations` - fetch a game with its full relations by id.
* `playmatch_get_game_file_history` - list the dat file imports a game file was
  seen in by game file id.
* `playmatch_list_companies` / `playmatch_get_company` - browse or fetch companies.
* `playmatch_list_platforms` / `playmatch_get_platform` - browse or fetch platforms.
* `playmatch_list_signature_groups` / `playmatch_get_signature_group` - browse or
  fetch signature groups (dat publishers such as No-Intro and Redump).
* `playmatch_list_dat_files` / `playmatch_get_dat_file` - browse or fetch dat files.
* `playmatch_list_dat_file_games` - list the games contained in a dat file.
* `playmatch_get_game_files` - fetch the files belonging to a game.
* `playmatch_search_games_by_name` - fuzzy game search by human title when you
  have no hash; returns candidate ids, names, and platforms.
* `playmatch_find_dats_containing_hash` - find the dat files that contain a hash.
* `playmatch_bulk_identify` - identify many ROMs in one call (capped at 100 items).

## Configuration

The service is configured through environment variables. The core runtime
variables and their defaults are:

* `PORT` - HTTP listener port. Defaults to `8080`. Also serves `/mcp`.
* `HTTP_WORKERS` - number of HTTP worker threads. Defaults to the CPU count.
* `METRICS_PORT` - Prometheus `/metrics` listener on a separate port. Defaults to
  `9090`.
* `DATABASE_MAX_CONNECTIONS` - database connection pool size. Defaults to `100`.
* `DATABASE_SQL_LOG_LEVEL` - log level filter for SQL statements. Defaults to
  `Debug`.
* `PARALLELISM` - worker count for import and processing jobs. Defaults to the
  CPU count.
* `INITIAL_DATA_INIT` - run the initial data import on first boot. Defaults to
  `true`.
* `FORCE_INITIAL_DATA_INIT` - force the initial data import even when it has
  already run. Defaults to `false`.
* `PLAYMATCH_DEFAULT_API_VERSION` - default API version for the `/api` alias.
  Defaults to `v1`.
* `TRUSTED_PROXY_CIDRS` - comma-separated CIDRs of the reverse proxies allowed to
  set the `CF-Connecting-IP` header that the per-IP rate limiter keys on. Defaults
  to Cloudflare's published ranges plus the private/loopback ranges (`10/8`,
  `172.16/12`, `192.168/16`, loopback, IPv6 ULA/link-local), so both a
  Cloudflare-fronted deployment and an app behind a reverse proxy on a private
  network (Traefik/nginx on the same Docker network, a k8s ingress) work with no
  configuration. Setting this replaces the defaults; use it to lock trust down to a
  specific proxy, or set it empty to trust no proxy and always key on the direct
  peer. A public client reaching the app directly has a public peer address that is
  not trusted, so it cannot spoof the header to mint a fresh rate-limit bucket.
* `TRUST_CF_CONNECTING_IP` - master switch for the above; set `false` to never
  honor `CF-Connecting-IP`. Defaults to `true` (still gated on the trusted-proxy
  check).

See [.env.example](.env.example) for the full list, including every metadata
provider's credentials and toggles.

## Deployment

Docker images are available [Here](https://github.com/RetroRealm/playmatch/pkgs/container/playmatch)

## Built With

* [Rust](https://www.rust-lang.org/) - The programming language used
* [tokio](https://tokio.rs/) - The async runtime used
* [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
* [serde](https://serde.rs/) - Serialization/Deserialization
* [actix-web](https://github.com/actix/actix-web) - The web framework used
* [SeaORM](https://www.sea-ql.org/SeaORM/) - The Database ORM used
* [Redis](https://redis.io/) / [Valkey](https://valkey.io/) / [DragonflyDB](https://www.dragonflydb.io/) - Used for caching and rate limiting

## Contributing

Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for details on our code of
conduct, and the process for submitting pull requests to us.

## Versioning

We use [SemVer](http://semver.org/) for versioning. For the versions available, see
the [tags on this repository](https://github.com/RetroRealm/playmatch/tags).

## Authors

* **DevYukine** - *Initial work* - [DevYukine](https://github.com/DevYukine)

See also the list of [contributors](https://github.com/RetroRealm/playmatch/contributors) who participated in this
project.

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details

## Acknowledgments

* [hasheous](https://github.com/gaseous-project/hasheous) - Another Project for the same goal
