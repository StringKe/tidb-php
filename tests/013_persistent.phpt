--TEST--
pdo_tidb: persistent connections are reused and liveness-checked
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$options = [PDO::ATTR_PERSISTENT => true, PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION];
$first = new Pdo\Tidb(tidb_dsn('idle_ping=0'), tidb_user(), tidb_password(), $options);
$id = $first->connectionId();
$first->exec('SET @kept = 1');
unset($first);

$second = new Pdo\Tidb(tidb_dsn('idle_ping=0'), tidb_user(), tidb_password(), $options);
var_dump($second->connectionId() === $id, $second->query('SELECT @kept')->fetchColumn());

tidb_connect()->exec('KILL CONNECTION '.$id);
unset($second);

$third = new Pdo\Tidb(tidb_dsn('idle_ping=0'), tidb_user(), tidb_password(), $options);
var_dump($third->connectionId() !== $id, $third->query('SELECT 1')->fetchColumn());
?>
--EXPECT--
bool(true)
int(1)
bool(true)
int(1)
