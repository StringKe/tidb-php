<?php

const TIDB_TEST_DATABASE = 'pdo_tidb_test';

function tidb_host(): string
{
    return getenv('PDO_TIDB_TEST_HOST') ?: '127.0.0.1';
}

function tidb_port(): int
{
    return (int) (getenv('PDO_TIDB_TEST_PORT') ?: 4400);
}

function tidb_user(): string
{
    return getenv('PDO_TIDB_TEST_USER') ?: 'root';
}

function tidb_password(): string
{
    return getenv('PDO_TIDB_TEST_PASSWORD') ?: '';
}

/** Directory with ca.pem, client.pem and client-key.pem of the server's CA; compose.yaml writes them to tests/certs. */
function tidb_certs(): string
{
    return getenv('PDO_TIDB_TEST_CERTS') ?: __DIR__.'/certs';
}

function tidb_dsn(string $extra = ''): string
{
    return 'tidb:host='.tidb_host().';port='.tidb_port().';sslmode=disable'.($extra === '' ? '' : ';'.$extra);
}

function tidb_connect(array $options = [], string $extra = ''): Pdo\Tidb
{
    $db = new Pdo\Tidb(tidb_dsn($extra), tidb_user(), tidb_password(), $options + [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION]);
    $db->exec('CREATE DATABASE IF NOT EXISTS '.TIDB_TEST_DATABASE);
    $db->exec('USE '.TIDB_TEST_DATABASE);
    return $db;
}

function tidb_error(callable $fn): void
{
    try {
        $fn();
        echo "no error\n";
    } catch (PDOException $e) {
        echo $e->getMessage(), "\n";
    }
}
