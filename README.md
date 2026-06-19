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

## MCP server

Playmatch ships a Model Context Protocol server so AI agents can identify ROMs
and browse the catalogue with the same data the public API serves. It is mounted
on the main API listener and speaks MCP over Streamable HTTP at `/mcp` (so the
endpoint is `http://<host>:<PORT>/mcp`). It shares the public API per-IP rate
limit.

It is controlled by one environment variable:

* `MCP_ENABLED` - set to `false` to disable the server. Defaults to `true`.

The server exposes these tools:

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

Please read [CONTRIBUTING.md](https://gist.github.com/PurpleBooth/b24679402957c63ec426) for details on our code of
conduct, and the process for submitting pull requests to us.

## Versioning

We use [SemVer](http://semver.org/) for versioning. For the versions available, see
the [tags on this repository](https://github.com/RetroRealm/playmatch/tags).

## Authors

* **DevYukine** - *Initial work* - [DevYukine](https://github.com/DevYukine)

See also the list of [contributors](https://github.com/RetroRealm/playmatch/contributors) who participated in this
project.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details

## Acknowledgments

* [hasheous](https://github.com/gaseous-project/hasheous) - Another Project for the same goal
