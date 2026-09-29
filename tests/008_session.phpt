--TEST--
pdo_tidb: ping, reset, database switching, init commands and session status helpers
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect([Pdo\Tidb::ATTR_INIT_COMMAND => "SET @init = 'ok'"], 'dbname=mysql');
var_dump($db->query('SELECT @init')->fetchColumn());
var_dump($db->ping(), str_starts_with($db->statistics(), 'Uptime:'));

$db->exec('SET @v = 1');
var_dump($db->resetConnection());
var_dump($db->query('SELECT @v')->fetchColumn(), $db->query('SELECT @init')->fetchColumn());
var_dump($db->query('SELECT DATABASE()')->fetchColumn());

var_dump($db->selectDatabase(TIDB_TEST_DATABASE), $db->query('SELECT DATABASE()')->fetchColumn());
tidb_error(fn () => $db->selectDatabase('no_such_database'));

$db->exec("SELECT CAST('x' AS SIGNED)");
var_dump($db->warningCount());
$db->exec('DROP TABLE IF EXISTS session_info');
$db->exec('CREATE TABLE session_info (id BIGINT PRIMARY KEY AUTO_RANDOM, n INT)');
$db->exec('INSERT INTO session_info (n) VALUES (1), (2)');
var_dump($db->lastInfo());
var_dump(($db->serverStatus() & 0x2) === 0x2);
var_dump($db->refresh());
$db->exec('DROP TABLE session_info');

$db->setAttribute(PDO::ATTR_EMULATE_PREPARES, false);
$st = $db->prepare('SELECT ?');
$st->execute([1]);
var_dump($db->resetConnection());
tidb_error(fn () => $st->execute([2]));
?>
--EXPECTF--
string(2) "ok"
bool(true)
bool(true)
bool(true)
NULL
string(2) "ok"
string(5) "mysql"
bool(true)
string(13) "pdo_tidb_test"
SQLSTATE[42000]: %s: 1049 Unknown database 'no_such_database'
int(1)
string(38) "Records: 2  Duplicates: 0  Warnings: 0"
bool(true)
bool(true)
bool(true)
SQLSTATE[HY000]: General error: %s
