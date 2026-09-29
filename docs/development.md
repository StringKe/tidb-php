# Development

## Layout

| Path | Content |
|---|---|
| `config.m4`, `Makefile.frag` | build definition |
| `package.xml` | PECL package definition |
| `composer.json` | package metadata read by PIE |
| `src/` | C sources, stub and generated arginfo |
| `ffi/` | Rust crate producing `libtidb_ffi.a` |
| `tests/` | `.phpt` tests and the TiDB configuration they use |
| `compose.yaml` | local TiDB with TLS for the tests |
| `docs/` | documentation |

Everything `phpize`, `configure`, `make` and `cargo` generate is ignored by git. `phpize --clean` removes the PHP build files, `cargo clean --manifest-path ffi/Cargo.toml` the Rust ones.

### Rust modules

| Module | Content |
|---|---|
| `packet`, `protocol` | packet framing and the OK, EOF, error, handshake and column definition packets |
| `io` | TCP and unix sockets, TLS upgrade, zlib and zstd compressed framing, timeouts |
| `tls` | rustls client configuration and the `sslmode` verification levels |
| `auth`, `handshake` | authentication plugins, handshake and `COM_CHANGE_USER` |
| `conn` | commands, result reading (buffered, streaming, cursor), `LOAD DATA LOCAL`, session helpers |
| `value` | text and binary row decoding, `COM_STMT_EXECUTE` parameter encoding |
| `dsn`, `lb` | DSN parsing, host failover and server discovery |
| `error` | client error codes, SQLSTATE mapping and TiDB error classes |
| `abi` | the C ABI used by `src/` |

## Local server

`compose.yaml` starts TiDB v8.5 with unistore storage. A one-shot container first writes a CA, a server certificate for `localhost` / `127.0.0.1` and a client certificate into `tests/certs/`, and TiDB serves TLS with them.

```sh
docker compose up -d --wait
```

| Endpoint | Address |
|---|---|
| MySQL protocol | `127.0.0.1:4400` |
| Status API | `127.0.0.1:10480` |

The user is `root` without a password. `docker compose down -v` removes everything; `tests/certs/` stays and is ignored by git.

## Build

```sh
phpize
./configure CFLAGS="-O2 -g -Wall -Wextra -Wno-unused-parameter -Wno-missing-field-initializers"
make
```

The build is expected to produce no compiler or linker warnings with these flags. `-Wpedantic` is not used because PHP's own headers trigger it. After changing `Makefile.frag` or `config.m4`, run `./configure` again; `./config.status` keeps the old fragment.

## Tests

```sh
make test NO_INTERACTION=1
```

The tests read their connection from the environment and default to the compose setup:

| Variable | Default |
|---|---|
| `PDO_TIDB_TEST_HOST` | `127.0.0.1` |
| `PDO_TIDB_TEST_PORT` | `4400` |
| `PDO_TIDB_TEST_USER` | `root` |
| `PDO_TIDB_TEST_PASSWORD` | empty |
| `PDO_TIDB_TEST_CERTS` | `tests/certs` |

Tests are skipped when the server cannot be reached, and the TLS and `changeUser()` tests when there are no certificates. They create the database `pdo_tidb_test` and a few users (`pdo_sm3`, `pdo_sha2`, `cu_*`), so point them at a disposable server only.

A single test runs with `make test TESTS=tests/011_tls_auth.phpt`.

Rust unit tests, formatting and lints:

```sh
cd ffi
cargo fmt --check
cargo clippy --release --all-targets --locked -- -D warnings
cargo test --release --locked
```

## Leak checks

On macOS, `leaks` checks both the Rust side and PHP's allocations:

```sh
MallocStackLogging=1 leaks --atExit -- php -d extension=modules/pdo_tidb.so script.php
USE_ZEND_ALLOC=0 MallocStackLogging=1 leaks --atExit -- php -d extension=modules/pdo_tidb.so script.php
```

On Linux, use `valgrind --leak-check=full` with `USE_ZEND_ALLOC=0`, or run the tests with `make test TESTS="-m"`.

## Changing the stub

`src/pdo_tidb.stub.php` is the source of `src/pdo_tidb_arginfo.h`. After editing it, regenerate the header with PHP's `gen_stub.php`:

```sh
php "$(php-config --prefix)/lib/php/build/gen_stub.php" src/pdo_tidb.stub.php
```

Distribution packages may install `gen_stub.php` elsewhere (for example `/usr/lib/php/<version>/build/`).

## Packaging

```sh
phpize --clean
pecl package package.xml
```

This writes `pdo_tidb-<version>.tgz` with exactly the files listed in `package.xml`. Before a release:

1. Update the version in `src/php_pdo_tidb.h`, `ffi/Cargo.toml`, `package.xml` and `CHANGELOG.md`.
2. Run the full test suite and the lints above.
3. Build the tarball, install it with `pecl install ./pdo_tidb-<version>.tgz` on a clean machine and run `php --ri pdo_tidb`.
4. Tag `v<version>`, push the tag and attach the tarball to the GitHub release.

## Commits

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) in English, for example `fix(ffi): read both OK packets of COM_REFRESH` or `docs: document cursor fetch size`. Scopes in use: `driver` (C sources), `ffi` (Rust crate), `build`, `tests`, `docs`.
