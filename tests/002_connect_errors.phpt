--TEST--
pdo_tidb: connection failures, wrong credentials and multi-host failover
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

tidb_error(fn () => new Pdo\Tidb('tidb:host=127.0.0.1;port=1;connect_timeout=2', tidb_user(), tidb_password()));
tidb_error(fn () => new Pdo\Tidb(tidb_dsn(), 'no_such_user', 'x'));
tidb_error(fn () => new Pdo\Tidb(tidb_dsn('dbname=no_such_database'), tidb_user(), tidb_password()));
tidb_error(fn () => new Pdo\Tidb(tidb_dsn('sslmode=bogus'), tidb_user(), tidb_password()));

$failover = new Pdo\Tidb('tidb:host=127.0.0.1:1,'.tidb_host().':'.tidb_port().';sslmode=disable;connect_timeout=2', tidb_user(), tidb_password());
var_dump($failover->connectionInfo()['endpoint'] === tidb_host().':'.tidb_port());
var_dump($failover->query('SELECT 1')->fetchColumn());

$named = new Pdo\Tidb(tidb_dsn('dbname=mysql'), tidb_user(), tidb_password());
var_dump($named->query('SELECT DATABASE()')->fetchColumn(), $named->connectionInfo()['database']);
?>
--EXPECTF--
SQLSTATE[08001] [2003] Can't connect to TiDB server on '127.0.0.1:1' (%s)
SQLSTATE[28000] [1045] Access denied for user 'no_such_user'@'%s' (using password: YES)
SQLSTATE[42000] [1049] Unknown database 'no_such_database'
%s
bool(true)
int(1)
string(5) "mysql"
string(5) "mysql"
