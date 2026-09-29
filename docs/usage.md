# Using pdo_tidb

## Connecting

```php
$db = new Pdo\Tidb('tidb:host=tidb.internal;port=4000;dbname=app', 'app', getenv('DB_PASSWORD'), [
    PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
]);
```

`new PDO('tidb:...')` works as well; since PHP 8.4 `PDO::connect('tidb:...')` returns a `Pdo\Tidb` instance.

### DSN

The DSN is `tidb:` followed by `key=value` pairs separated by `;`. Unknown keys are rejected, so a typo fails at connect time instead of being ignored.

| Key | Default | Meaning |
|---|---|---|
| `host` | `127.0.0.1` | One host, or several as `a:4000,b:4000,[::1]:4000`. Hosts without a port use `port`. |
| `port` | `4000` | Port for hosts that do not name one. |
| `unix_socket` | not set | Path of a local socket; replaces `host`. TLS is off by default on a socket. |
| `dbname`, `database` | not set | Default database. |
| `user`, `password` | not set | Used when the PDO constructor arguments are empty. |
| `charset` | `utf8mb4` | Connection character set. |
| `collation` | charset default | `utf8mb4_bin`, `utf8mb4_general_ci`, `utf8mb4_unicode_ci` and `utf8mb4_0900_ai_ci` are set in the handshake; others with `SET NAMES` after it. |
| `sslmode` | `preferred`, `verify_identity` on `*.tidbcloud.com` | `disable`, `preferred`, `required`, `verify_ca`, `verify_identity`. See [TLS](#tls). |
| `ssl_ca`, `ssl_capath` | system trust store | CA file, or directory of PEM files, replacing the system trust store. |
| `ssl_cert`, `ssl_key` | not set | Client certificate and key (PEM), both or neither. |
| `tls_version` | `TLSv1.2` | Lowest accepted version, `TLSv1.2` or `TLSv1.3`. |
| `compression` | off | `zlib` or `zstd`. |
| `zstd_level` | `3` | 1 to 22. |
| `connect_timeout`, `read_timeout`, `write_timeout` | none | Seconds, fractions allowed. |
| `multi_statements` | `1` | Send the `MULTI_STATEMENTS` capability. Without it TiDB rejects `SELECT 1; SELECT 2` with error 8130 unless `tidb_multi_statement_mode` is on. |
| `found_rows` | `0` | `rowCount()` of an `UPDATE` reports matched instead of changed rows. |
| `local_infile` | `0` | Allow `LOAD DATA LOCAL INFILE` to read any file the PHP process can read. |
| `local_infile_directory` | not set | Allow it only for files under this directory; implies `local_infile`. |
| `init_command` | not set | SQL run after every connect and every `resetConnection()`. |
| `time_zone`, `max_execution_time`, `idle_transaction_timeout` | not set | Session variables set after connecting (`idle_transaction_timeout` sets `tidb_idle_transaction_timeout`). |
| `session.<name>` | not set | Any other session variable, for example `session.tidb_txn_mode=pessimistic`. |
| `attr.<name>` | not set | Extra connection attribute, visible in `INFORMATION_SCHEMA.PROCESSLIST` tooling and audit logs. |
| `discover` | `0` | Replace the configured hosts with all TiDB servers from `INFORMATION_SCHEMA.TIDB_SERVERS_INFO`. |
| `discover_ttl` | `60` | Seconds a discovered server list is reused within the process. |
| `max_lifetime` | none | Seconds after which a persistent connection is replaced instead of reused. |
| `idle_ping` | ping on every reuse | Seconds a persistent connection may stay idle before its reuse sends a `COM_PING`; `0` pings on every reuse. |
| `auth_plugin` | server default | Plugin for the first authentication response, for example `mysql_clear_password` for `tidb_auth_token`. |

### Hosts and failover

With several hosts the driver tries them in random order. A host that refused a connection is moved to the end of the list for 30 seconds in the whole process, so the next connects go to the healthy servers first. With `discover=1` the first successful connection reads the list of TiDB servers and later connects in the same process use it until `discover_ttl` expires. Discovery errors do not fail the connection; `connectionInfo()['discovery_error']` reports them.

Put a load balancer in front of TiDB when the servers are not reachable directly, as with TiDB Cloud.

## Attributes

Driver options passed to the constructor are applied on top of the DSN; most of them are connection settings and have a DSN key. Attributes marked "runtime" can also be changed with `setAttribute()`.

| Attribute | DSN key | Runtime | Meaning |
|---|---|---|---|
| `PDO::ATTR_TIMEOUT` | `connect_timeout` | no | Connect timeout in seconds. |
| `PDO::ATTR_EMULATE_PREPARES` | | yes | Default `true`. `false` uses `COM_STMT_PREPARE`; statements TiDB cannot prepare fall back to emulation. |
| `PDO::ATTR_AUTOCOMMIT` | | yes | Runs `SET autocommit`. |
| `PDO::ATTR_FETCH_TABLE_NAMES` | | yes | Column names become `table.column`. |
| `Pdo\Tidb::ATTR_BUFFERED_QUERY` | | yes | Default `true`. `false` streams rows from the socket; see [Reading results](#reading-results). |
| `Pdo\Tidb::ATTR_CURSOR_FETCH_SIZE` | | yes, also per statement | Rows per `COM_STMT_FETCH`; `0` turns server-side cursors off. Only native prepares use cursors. |
| `Pdo\Tidb::ATTR_MULTI_STATEMENTS` | `multi_statements` | yes | Switches the capability with `COM_SET_OPTION`. |
| `Pdo\Tidb::ATTR_FOUND_ROWS` | `found_rows` | no | |
| `Pdo\Tidb::ATTR_LOCAL_INFILE` | `local_infile` | no | |
| `Pdo\Tidb::ATTR_LOCAL_INFILE_DIRECTORY` | `local_infile_directory` | no | |
| `Pdo\Tidb::ATTR_INIT_COMMAND` | `init_command` | no | |
| `Pdo\Tidb::ATTR_SSL_MODE` | `sslmode` | no | |
| `Pdo\Tidb::ATTR_SSL_CA`, `ATTR_SSL_CAPATH`, `ATTR_SSL_CERT`, `ATTR_SSL_KEY` | `ssl_*` | no | |
| `Pdo\Tidb::ATTR_TLS_VERSION` | `tls_version` | no | Reading it returns the negotiated version, or `null` without TLS. |
| `Pdo\Tidb::ATTR_COMPRESSION` | `compression` | no | Reading it returns `zlib`, `zstd` or `null`. |
| `Pdo\Tidb::ATTR_ZSTD_LEVEL` | `zstd_level` | no | |
| `Pdo\Tidb::ATTR_READ_TIMEOUT`, `ATTR_WRITE_TIMEOUT` | `read_timeout`, `write_timeout` | no | |
| `Pdo\Tidb::ATTR_DISCOVER` | `discover` | no | |
| `Pdo\Tidb::ATTR_MAX_LIFETIME`, `ATTR_IDLE_PING` | `max_lifetime`, `idle_ping` | no | |
| `Pdo\Tidb::ATTR_AUTH_PLUGIN` | `auth_plugin` | no | Reading it returns the plugin that authenticated the session. |

`PDO::ATTR_SERVER_VERSION` returns the full TiDB version string (`8.0.11-TiDB-v8.5.8`), `PDO::ATTR_SERVER_INFO` the `COM_STATISTICS` line, `PDO::ATTR_CONNECTION_STATUS` `host:port via TCP/IP` or `via TLS`.

## Pdo\Tidb methods

| Method | Meaning |
|---|---|
| `ping(): bool` | `COM_PING`. |
| `resetConnection(): bool` | `COM_RESET_CONNECTION`: user variables, session variables, temporary tables and prepared statements are dropped, then `init_command` and the DSN session variables run again. A database chosen with SQL `USE` is lost; one chosen in the DSN or with `selectDatabase()` stays. |
| `changeUser(string $user, ?string $password = null, ?string $database = null): bool` | `COM_CHANGE_USER`. When the server rejects it, the connection keeps the previous user. |
| `selectDatabase(string $name): bool` | `COM_INIT_DB`. |
| `statistics(): string\|false` | `COM_STATISTICS`. |
| `refresh(int $options = Pdo\Tidb::REFRESH_GRANT): bool` | `COM_REFRESH`; TiDB implements `REFRESH_GRANT` (`FLUSH PRIVILEGES`). |
| `connectionId(): int` | Server connection id. |
| `killQuery(): bool` | Opens a second connection to the same TiDB server, with the same credentials, and runs `KILL QUERY` for this connection's id. |
| `listFields(string $table, string $wildcard = ''): array\|false` | `COM_FIELD_LIST`, as `getColumnMeta()` arrays. TiDB ignores the wildcard, so the driver filters the names with `LIKE` semantics itself. |
| `loadDataLocal(string $sql, string $data): int\|false` | Runs a `LOAD DATA LOCAL INFILE` statement and answers the file request with `$data`, whatever file name the statement contains. Needs no `local_infile` setting. Returns the affected rows. |
| `warningCount(): int` | Warnings of the last statement. |
| `lastInfo(): ?string` | Info string of the last statement, such as `Records: 2  Duplicates: 0  Warnings: 0`. |
| `serverStatus(): int` | Server status flags of the last response. |
| `connectionInfo(): array` | `endpoint`, `server_version`, `connection_id`, `database`, `auth_plugin`, `tls_version`, `tls_cipher`, `compression`, `capabilities`, `status`, `discovery_error`. |
| `static errorClass(int $code, bool $connectionLost = false): string` | See [Errors and retries](#errors-and-retries). |

## Types

Rows arrive in the text protocol with emulated prepares and in the binary protocol with native prepares. Both produce the same PHP values:

| TiDB type | PHP value |
|---|---|
| `TINYINT` to `BIGINT`, `YEAR` | `int`; `BIGINT UNSIGNED` above `PHP_INT_MAX` as a decimal `string` |
| `FLOAT`, `DOUBLE` | `float` |
| `DECIMAL` | `string` with the column scale, never a float |
| `DATE`, `DATETIME(n)`, `TIMESTAMP(n)`, `TIME(n)` | `string` in MySQL format with the fractional digits of the column (`2026-09-29 10:00:00.123456`, `-01:02:03.500`) |
| `CHAR`, `VARCHAR`, `TEXT`, `JSON`, `ENUM`, `SET` | `string` |
| `BINARY`, `VARBINARY`, `BLOB`, `BIT` | `string` of raw bytes |
| `NULL` | `null` |

`getColumnMeta()` returns `native_type`, `pdo_type` (`PARAM_INT` for integer types, `PARAM_LOB` for binary BLOB types, `PARAM_STR` otherwise), `flags`, `table`, `name`, `len`, `precision`, plus `tidb:org_name`, `tidb:org_table`, `tidb:schema` and `tidb:charset`. TiDB does not set the `blob` flag on column definitions.

## Parameters

`?` and `:name` placeholders work in both prepare modes, and a `:name` may appear several times. With native prepares, integers and booleans are sent as integers, floats as doubles, strings as bytes, and `PARAM_LOB` values (strings or streams) with `COM_STMT_SEND_LONG_DATA` in 8 MiB chunks; a 20 MiB parameter is covered by the tests. TiDB limits a single row entry (`txn-entry-size-limit`, 6 MiB by default) and a `BLOB` column to 64 KiB; use `LONGBLOB` for large values.

`quote()` escapes with backslashes, or doubles single quotes when the session has `NO_BACKSLASH_ESCAPES`. `quote($value, PDO::PARAM_LOB)` returns a `_binary'...'` literal, and SQL is sent as bytes, so binary literals survive.

## Reading results

- **Buffered** (default): the whole result is read at `execute()`; `rowCount()` is the number of rows, and the connection is free for the next query immediately.
- **Unbuffered** (`ATTR_BUFFERED_QUERY => false`): rows are read while you fetch. Until the statement is fetched to the end or `closeCursor()` is called, another query on the same connection fails with error 2014, as in pdo_mysql. `rowCount()` counts the rows fetched so far.
- **Server-side cursor** (native prepares with `ATTR_CURSOR_FETCH_SIZE > 0`): TiDB keeps the result and the driver fetches it in batches with `COM_STMT_FETCH`. Other queries can run between fetches. Cursors bound the memory on the PHP side; how much the server holds depends on the TiDB version and its cursor settings.

Multi-statement queries return one result per statement; move with `nextRowset()`.

## Transactions

`beginTransaction()`, `commit()` and `rollBack()` send `BEGIN`, `COMMIT` and `ROLLBACK`. `inTransaction()` reads the server status flags, so it also reflects `BEGIN PESSIMISTIC`, `START TRANSACTION READ ONLY` or `autocommit=0` sessions. To choose the mode per transaction, send it as SQL:

```php
$db->exec('BEGIN PESSIMISTIC');
// ...
$db->commit();
```

## Errors and retries

`errorInfo()` and `PDOException::$errorInfo` hold the SQLSTATE, the TiDB error code and the message. Client-side errors use the MySQL client codes: 2003 cannot connect, 2006 the connection is gone, 2013 the connection was lost during a query, 2014 an unbuffered result is still open, 2026 TLS failure, 2061 authentication plugin problem, 2068 a refused `LOAD DATA LOCAL` file.

After 2006 or 2013 the connection is closed for good; create a new one. `Pdo\Tidb::errorClass()` maps a code to what the caller should do:

| Class | Codes | Action |
|---|---|---|
| `retry_transaction` | 8002, 8005, 8022, 8028, 9007, 1213, 1205 | Roll back and run the whole transaction again. |
| `backoff` | 9001 to 9005, 9008, 9010, 8027, 8252 | The cluster is busy or a region moved; wait and retry. |
| `busy` | 3572 | `NOWAIT` found a lock; retry later or report. |
| `retry_after_unlock` | 8020 | A table lock blocks; retry after it is released. |
| `fatal` | 8004, 8025, 8141, 8147, 8200, 8216, 9006 | Retrying the same statement does not help (entry too large, memory quota, schema changed during commit and similar). |
| `reconnect` | 2002, 2003, 2006, 2013, or any code with `$connectionLost = true` | Reconnect before retrying. |
| `none` | everything else | Not a transient condition. |

```php
for ($attempt = 1; ; $attempt++) {
    try {
        $db->beginTransaction();
        transfer($db);
        $db->commit();
        break;
    } catch (PDOException $e) {
        if ($db->inTransaction()) {
            $db->rollBack();
        }
        $class = Pdo\Tidb::errorClass((int) $e->errorInfo[1]);
        if ($attempt >= 5 || !in_array($class, ['retry_transaction', 'backoff'], true)) {
            throw $e;
        }
        usleep(50_000 * $attempt);
    }
}
```

## TLS

| `sslmode` | Encryption | Certificate chain | Host name |
|---|---|---|---|
| `disable` | no | | |
| `preferred` | when the server offers TLS | not checked without `ssl_ca` | no |
| `required` | yes, fails otherwise | not checked without `ssl_ca` | no |
| `verify_ca` | yes | checked | no |
| `verify_identity` | yes | checked | checked |

Without `ssl_ca` / `ssl_capath`, `verify_ca` and `verify_identity` use the operating system's trust store, which is what TiDB Cloud needs. Hosts ending in `.tidbcloud.com` default to `verify_identity`.

## Authentication

| Server plugin | What the driver sends |
|---|---|
| `mysql_native_password` | SHA1 scramble. |
| `caching_sha2_password`, `tidb_sm3_password` | TiDB always asks for full authentication, which sends the password in clear text; the driver refuses to do that without TLS or a unix socket (error 2061). |
| `tidb_auth_token` | The token as a clear-text password over TLS; pass it as the password. |
| `authentication_ldap_simple` | Clear-text password over TLS. |
| `authentication_ldap_sasl` | SCRAM-SHA-1 or SCRAM-SHA-256 exchange. |
| `auth_socket` | Nothing; the server reads the OS user of the unix socket peer. |

GSSAPI (`authentication_ldap_sasl` with GSSAPI) is not implemented. The test suite covers `mysql_native_password`, `caching_sha2_password` and `tidb_sm3_password` against a real TiDB, including `changeUser()`; `tidb_auth_token`, the LDAP plugins and `auth_socket` follow TiDB's server code and have unit tests for the SCRAM exchange, but need a JWKS, an LDAP server or a unix socket to test end to end.

## LOAD DATA LOCAL

```php
$db->loadDataLocal("LOAD DATA LOCAL INFILE 'rows.csv' INTO TABLE t FIELDS TERMINATED BY ',' (a, b)", "1,x\n2,y\n");
```

Plain `exec('LOAD DATA LOCAL INFILE ...')` reads the file named by the server's request, and only when `local_infile` or `local_infile_directory` allows it. The server chooses the file name it asks for, so on shared or untrusted servers use `local_infile_directory` or `loadDataLocal()`.

## Persistent connections and workers

With `PDO::ATTR_PERSISTENT`, a reused connection is checked before it is handed out: a broken connection or one with an unread result is dropped, one older than `max_lifetime` is replaced, and one idle for `idle_ping` seconds or more is pinged. `resetConnection()` returns a connection to a clean session state between jobs in a long-running worker.

## Server quirks handled by the driver

- A connection that has not run any command cannot be the target of `KILL` (TiDB answers 1105 with a nil pointer error). The driver sends a `COM_PING` after connecting when nothing else runs, so `KILL` and `killQuery()` always work.
- `COM_REFRESH` with `REFRESH_GRANT` is answered with two OK packets; the driver reads both.
- `COM_FIELD_LIST` ignores the wildcard; the driver applies it.
- `COM_CHANGE_USER` skips the full-authentication round for `caching_sha2_password` and `tidb_sm3_password` and compares the response with the clear-text password; the driver sends the password directly there, over TLS only.
- With protocol compression, every server flush restarts the inner packet sequence at the compressed sequence, so the inner sequence cannot be predicted across result sets. The driver checks the compressed sequence and follows the inner one.
- Column definitions never carry `BLOB_FLAG`; the driver reports `PARAM_LOB` from the BLOB type ids with the binary charset.
