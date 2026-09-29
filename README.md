# tidb-php

`pdo_tidb` is a native PDO driver for [TiDB](https://github.com/pingcap/tidb). TiDB speaks the MySQL wire protocol, but it is not MySQL: it has its own authentication plugins, its own error codes for transaction conflicts, zstd compression, server-side cursors with different rules and a set of server quirks. This driver implements the protocol in Rust against TiDB's behaviour and exposes it as a PDO driver plus a `Pdo\Tidb` subclass.

```php
$db = new PDO('tidb:host=127.0.0.1;port=4000;dbname=test', 'root', '');
$db->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

$st = $db->prepare('SELECT id, name FROM users WHERE id = :id');
$st->execute(['id' => 42]);
var_dump($st->fetch(PDO::FETCH_ASSOC));
```

## Features

- MySQL protocol 4.1 with `DEPRECATE_EOF`, written in Rust with no MySQL client library.
- Authentication: `mysql_native_password`, `caching_sha2_password`, `tidb_sm3_password`, `mysql_clear_password` (used by `tidb_auth_token` and LDAP simple), `authentication_ldap_sasl` (SCRAM-SHA-1 and SCRAM-SHA-256), `auth_socket`, and `COM_CHANGE_USER` for all of them.
- TLS through rustls with `disable`, `preferred`, `required`, `verify_ca` and `verify_identity`, client certificates and the system trust store; `verify_identity` is the default for TiDB Cloud hosts.
- zlib and zstd protocol compression with a configurable zstd level.
- Emulated and native prepared statements, repeated named placeholders, `PARAM_LOB` streams sent in chunks with `COM_STMT_SEND_LONG_DATA`.
- Buffered, unbuffered and server-side cursor (`COM_STMT_FETCH`) result reading; multi-statement queries with `nextRowset()`.
- `inTransaction()` from the server status flags, so it is correct after `BEGIN PESSIMISTIC` or `START TRANSACTION` issued as SQL.
- TiDB error classes (`Pdo\Tidb::errorClass()`): write conflicts, lock waits, region and TiKV back-off, memory quota and connection loss, to decide between retrying the transaction, backing off, and reconnecting.
- Several hosts in one DSN with failover, optional discovery of all TiDB servers from `INFORMATION_SCHEMA.TIDB_SERVERS_INFO`, connection lifetime and idle ping for persistent connections.
- `LOAD DATA LOCAL` from PHP strings or from files restricted to a directory.
- `Pdo\Tidb` methods: `ping()`, `resetConnection()`, `changeUser()`, `selectDatabase()`, `killQuery()`, `listFields()`, `loadDataLocal()`, `statistics()`, `refresh()`, warning count, last info, server status and connection details.

## Requirements

- PHP 8.4 or newer with `pdo`. Development and the test suite run on PHP 8.5; 8.4 builds against the same API but is not covered by the tests yet.
- Rust 1.88 or newer (`cargo` on `PATH`) and network access to crates.io on the first build. A prebuilt `libtidb_ffi.a` can be passed instead, see [docs/building.md](docs/building.md).
- A C toolchain, `autoconf` and `phpize` (the `php-dev` / `php-devel` package on Linux).
- macOS or Linux. The tests run against TiDB v8.5; older TiDB versions are not covered.

## Install

With PECL, from a release tarball:

```sh
pecl install https://github.com/stringke/tidb-php/releases/download/v0.1.0/pdo_tidb-0.1.0.tgz
```

With [PIE](https://github.com/php/pie):

```sh
pie install stringke/tidb-php
```

From source:

```sh
git clone https://github.com/stringke/tidb-php.git
cd tidb-php
phpize
./configure
make
make install
```

Then enable the extension after `pdo`:

```ini
extension=pdo_tidb
```

`php -m | grep pdo_tidb` and `php -r 'var_dump(PDO::getAvailableDrivers());'` confirm it is loaded. More options, prebuilt libraries and troubleshooting are in [docs/building.md](docs/building.md).

## Documentation

- [docs/usage.md](docs/usage.md): DSN, attributes, `Pdo\Tidb` methods, type mapping, cursors, errors and retries, TLS and authentication, long-running workers.
- [docs/building.md](docs/building.md): every install path, configure options and troubleshooting.
- [docs/development.md](docs/development.md): repository layout, local TiDB, tests and leak checks.
- [CHANGELOG.md](CHANGELOG.md)

## License

Apache License 2.0, see [LICENSE](LICENSE).
