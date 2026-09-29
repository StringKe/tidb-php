# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-09-29

First release.

### Added

- PDO driver `tidb` for TiDB, with the MySQL 4.1 protocol and `DEPRECATE_EOF` implemented in Rust, without a MySQL client library.
- Authentication with `mysql_native_password`, `caching_sha2_password`, `tidb_sm3_password`, `mysql_clear_password`, `authentication_ldap_sasl` (SCRAM-SHA-1, SCRAM-SHA-256) and `auth_socket`, and `COM_CHANGE_USER` for each of them.
- TLS through rustls with the `disable`, `preferred`, `required`, `verify_ca` and `verify_identity` modes, client certificates and the system trust store.
- zlib and zstd protocol compression with a configurable zstd level.
- Emulated and native prepared statements, repeated named placeholders and `PARAM_LOB` streams sent with `COM_STMT_SEND_LONG_DATA`.
- Buffered, unbuffered and server-side cursor result reading, and multi-statement queries with `nextRowset()`.
- TiDB error classes through `Pdo\Tidb::errorClass()` for write conflicts, lock waits, back-off, memory quota and connection loss.
- Host failover, server discovery from `INFORMATION_SCHEMA.TIDB_SERVERS_INFO`, connection lifetime and idle ping for persistent connections.
- `LOAD DATA LOCAL` from PHP strings or from files inside an allowed directory.
- `Pdo\Tidb` methods `ping()`, `resetConnection()`, `changeUser()`, `selectDatabase()`, `killQuery()`, `listFields()`, `loadDataLocal()`, `statistics()` and `refresh()`, plus warning count, last info, server status and connection details.
- Handling of TiDB server behaviour: the two OK packets of `COM_REFRESH`, `COM_FIELD_LIST` ignoring its wildcard, `COM_CHANGE_USER` skipping the full-authentication round, the inner packet sequence restarting after each flush under compression, and column definitions without `BLOB_FLAG`.
- PECL `package.xml` and PIE `composer.json`.

[0.1.0]: https://github.com/stringke/tidb-php/releases/tag/v0.1.0
