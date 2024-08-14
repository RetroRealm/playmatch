# Playmatch

A microservice for matching ROM file hashes and retrieving and caching game metadata. Ensures fast and accurate ROM
identification
and efficient metadata retrieval, made for RetroRealm but offers an api which any Rom Manager or Archival System can
use.

The public API is available at [playmatch.retrorealm.dev](https://playmatch.retrorealm.dev/swagger-ui/)

## Getting Started

### Prerequisites

1. Rust 1.80+ from [here](https://www.rust-lang.org/tools/install)
2. PostgreSQL 12+ from [here](https://www.postgresql.org/download/)

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
