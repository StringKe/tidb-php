<?php

/** @generate-class-entries */

namespace Pdo;

/**
 * @strict-properties
 * @not-serializable
 */
class Tidb extends \PDO
{
    /** @cvalue PDO_TIDB_ATTR_BUFFERED_QUERY */
    public const int ATTR_BUFFERED_QUERY = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_CURSOR_FETCH_SIZE */
    public const int ATTR_CURSOR_FETCH_SIZE = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_MULTI_STATEMENTS */
    public const int ATTR_MULTI_STATEMENTS = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_FOUND_ROWS */
    public const int ATTR_FOUND_ROWS = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_LOCAL_INFILE */
    public const int ATTR_LOCAL_INFILE = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_LOCAL_INFILE_DIRECTORY */
    public const int ATTR_LOCAL_INFILE_DIRECTORY = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_INIT_COMMAND */
    public const int ATTR_INIT_COMMAND = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_SSL_MODE */
    public const int ATTR_SSL_MODE = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_SSL_CA */
    public const int ATTR_SSL_CA = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_SSL_CAPATH */
    public const int ATTR_SSL_CAPATH = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_SSL_CERT */
    public const int ATTR_SSL_CERT = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_SSL_KEY */
    public const int ATTR_SSL_KEY = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_TLS_VERSION */
    public const int ATTR_TLS_VERSION = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_COMPRESSION */
    public const int ATTR_COMPRESSION = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_ZSTD_LEVEL */
    public const int ATTR_ZSTD_LEVEL = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_READ_TIMEOUT */
    public const int ATTR_READ_TIMEOUT = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_WRITE_TIMEOUT */
    public const int ATTR_WRITE_TIMEOUT = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_DISCOVER */
    public const int ATTR_DISCOVER = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_MAX_LIFETIME */
    public const int ATTR_MAX_LIFETIME = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_IDLE_PING */
    public const int ATTR_IDLE_PING = UNKNOWN;

    /** @cvalue PDO_TIDB_ATTR_AUTH_PLUGIN */
    public const int ATTR_AUTH_PLUGIN = UNKNOWN;

    public const int REFRESH_GRANT = 1;

    public function ping(): bool {}

    public function resetConnection(): bool {}

    public function changeUser(string $user, ?string $password = null, ?string $database = null): bool {}

    public function selectDatabase(string $database): bool {}

    public function statistics(): string|false {}

    public function refresh(int $options = Tidb::REFRESH_GRANT): bool {}

    public function connectionId(): int {}

    public function killQuery(): bool {}

    public function listFields(string $table, string $wildcard = ""): array|false {}

    public function loadDataLocal(string $sql, string $data): int|false {}

    public function warningCount(): int {}

    public function lastInfo(): ?string {}

    public function serverStatus(): int {}

    public function connectionInfo(): array {}

    public static function errorClass(int $code, bool $connectionLost = false): string {}
}
