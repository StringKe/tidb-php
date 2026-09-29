--TEST--
pdo_tidb: multi-statement queries, nextRowset and rowCount per result
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
$db->exec('DROP TABLE IF EXISTS multi');
$db->exec('CREATE TABLE multi (id BIGINT PRIMARY KEY AUTO_RANDOM, n INT)');
$db->exec('INSERT INTO multi (n) VALUES (1), (1), (2)');

$st = $db->query('SELECT 1 AS a; SELECT n FROM multi WHERE n = 2; UPDATE multi SET n = 3 WHERE n = 1');
do {
    var_dump($st->columnCount(), $st->fetchAll(PDO::FETCH_NUM), $st->rowCount());
} while ($st->nextRowset());

$db->setAttribute(Pdo\Tidb::ATTR_MULTI_STATEMENTS, false);
tidb_error(fn () => $db->query('SELECT 1; SELECT 2'));
$db->setAttribute(Pdo\Tidb::ATTR_MULTI_STATEMENTS, true);
var_dump($db->exec('DELETE FROM multi WHERE n = 3; DELETE FROM multi'));
$db->exec('DROP TABLE multi');
?>
--EXPECTF--
int(1)
array(1) {
  [0]=>
  array(1) {
    [0]=>
    int(1)
  }
}
int(1)
int(1)
array(1) {
  [0]=>
  array(1) {
    [0]=>
    int(2)
  }
}
int(1)
int(0)
array(0) {
}
int(2)
SQLSTATE[HY000]: General error: 8130 %s
int(2)
