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
