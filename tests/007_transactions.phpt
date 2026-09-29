--TEST--
pdo_tidb: transactions, server-driven inTransaction and autocommit
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
$db->exec('DROP TABLE IF EXISTS tx');
$db->exec('CREATE TABLE tx (id BIGINT PRIMARY KEY AUTO_RANDOM, n INT)');

$db->beginTransaction();
var_dump($db->inTransaction());
$db->exec('INSERT INTO tx (n) VALUES (1)');
$db->rollBack();
var_dump($db->inTransaction(), $db->query('SELECT COUNT(*) FROM tx')->fetchColumn());

$db->beginTransaction();
$db->exec('INSERT INTO tx (n) VALUES (2)');
$db->commit();
var_dump($db->query('SELECT COUNT(*) FROM tx')->fetchColumn());

$db->exec('BEGIN PESSIMISTIC');
var_dump($db->inTransaction());
$db->exec('SELECT n FROM tx FOR UPDATE');
$db->commit();
var_dump($db->inTransaction());

tidb_error(fn () => $db->commit());

$manual = tidb_connect([PDO::ATTR_AUTOCOMMIT => false]);
var_dump($manual->getAttribute(PDO::ATTR_AUTOCOMMIT), $manual->query('SELECT @@autocommit')->fetchColumn());
$manual->exec('INSERT INTO tx (n) VALUES (3)');
var_dump($manual->inTransaction());
$manual->exec('ROLLBACK');
$manual->setAttribute(PDO::ATTR_AUTOCOMMIT, true);
var_dump($manual->query('SELECT @@autocommit')->fetchColumn(), $db->query('SELECT COUNT(*) FROM tx')->fetchColumn());
$db->exec('DROP TABLE tx');
?>
--EXPECT--
bool(true)
bool(false)
int(0)
int(1)
bool(true)
bool(false)
There is no active transaction
bool(false)
int(0)
bool(true)
int(1)
int(1)
