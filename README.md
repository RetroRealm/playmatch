<div align="center">
  <picture>
    <source srcset=".github/resources/Playmatch%20Inverted%20Color%20Transparent%20bg.svg" media="(prefers-color-scheme: dark)">
    <source srcset=".github/resources/Playmatch%20Main%20Logo%20Transparent%20bg.svg" media="(prefers-color-scheme: light)">
    <img src=".github/resources/Playmatch%20Main%20Logo%20Transparent%20bg.svg" height="180" alt="Playmatch logo">
  </picture>
  <h3>ROM identification and game metadata service</h3>
</div>

Playmatch identifies ROM files by hash or file name and serves cached game
metadata for them. It imports DAT files from No-Intro, Redump and community
sources daily, matches games against IGDB, SteamGridDB, ScreenScraper,
MobyGames, LaunchBox, OpenVGDB, EmuReady, RetroAchievements, TheGamesDB and
Hasheous, and exposes the result over a versioned HTTP API and an MCP
server. The public instance runs at
[playmatch.retrorealm.dev](https://playmatch.retrorealm.dev/swagger-ui/).

## How it works

Identification resolves the strongest available hash first (sha256, then
sha1, md5, crc with size) and falls back to file name plus size. Matches are
cached in Redis; game, platform and company records live in PostgreSQL and
are linked to each metadata provider by background match cycles.

## Workspace layout

- `src/` - the `playmatch` server binary
- `api/` - actix-web routes, OpenAPI documents, middleware
- `service/` - domain logic: DAT ingestion, identify cascade, providers
- `entity/` - generated SeaORM entities
- `migration/` - SeaORM database migrations
- `mcp/` - MCP server tools and discovery card

## Getting started

You need Rust 1.95+, PostgreSQL 18+ and a Redis-compatible server such as
[Redis](https://redis.io/), [Valkey](https://valkey.io/) or
[DragonflyDB](https://www.dragonflydb.io/).

1. Clone the repository
2. Rename `.env.example` to `.env` and fill in the required variables
3. `cargo run --package playmatch --bin playmatch`

Configuration is environment variables only; `.env.example` documents every
variable, including provider credentials, rate-limit trust settings and
tuning knobs. Test with `cargo test --workspace`; the integration tests
start throwaway PostgreSQL and Redis containers through testcontainers, so
a running Docker daemon is required.

## HTTP API

The API is versioned under `/api/v1` (frozen) and `/api/v2` (additive; full
v1 parity plus pagination, bulk and stats endpoints). The bare `/api` prefix
mirrors the version set by `PLAYMATCH_DEFAULT_API_VERSION`, and
`GET /api/versions` lists the available versions. Swagger UI is served at
`/swagger-ui/` with one tab per document. The full reference for versioning,
pagination, bulk endpoints, authentication, errors and identify semantics is
in [docs/api.md](docs/api.md).

## MCP server

An MCP server with 18 tools for identifying and browsing the catalogue is
mounted at `/mcp` (Streamable HTTP) unless `MCP_ENABLED=false`, with
discovery documents at `/.well-known/mcp-server-card` and
`/.well-known/mcp.json`. Details are in [docs/api.md](docs/api.md).

## Deployment

Container images are published to
[GHCR](https://github.com/RetroRealm/playmatch/pkgs/container/playmatch).
The server needs PostgreSQL, a Redis-compatible store, and the environment
from `.env.example`. Cron imports and the MCP session manager run in
process, so run a single instance.

## Contributing

Read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before participating. Pull
requests run the CI gates: rustfmt, clippy, the test suite and a security
audit.

## Versioning

Releases follow [SemVer](http://semver.org/); see the
[tags on this repository](https://github.com/RetroRealm/playmatch/tags). The
HTTP API is versioned independently of the crate version.

## License

Playmatch is licensed under the [MIT license](LICENSE.md).

## Acknowledgments

- [Hasheous](https://github.com/gaseous-project/hasheous) - another project with the same goal
