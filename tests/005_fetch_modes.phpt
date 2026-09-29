--TEST--
pdo_tidb: buffered, unbuffered and server-side cursor result reading
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
$db->exec('DROP TABLE IF EXISTS fetches');
$db->exec('CREATE TABLE fetches (id BIGINT PRIMARY KEY AUTO_RANDOM, n INT)');
$db->exec('INSERT INTO fetches (n) VALUES '.implode(',', array_map(fn ($i) => "($i)", range(1, 50))));
$sql = 'SELECT n FROM fetches ORDER BY n';

$buffered = $db->query($sql);
var_dump($buffered->rowCount(), $db->query('SELECT 1')->fetchColumn());

$db->setAttribute(Pdo\Tidb::ATTR_BUFFERED_QUERY, false);
$unbuffered = $db->query($sql);
var_dump($unbuffered->fetchColumn(), $unbuffered->rowCount());
tidb_error(fn () => $db->query('SELECT 1'));
$unbuffered->closeCursor();
var_dump($db->query('SELECT 2')->fetchColumn());
$unbuffered = $db->query($sql);
var_dump(count($unbuffered->fetchAll()), $unbuffered->rowCount());
$db->setAttribute(Pdo\Tidb::ATTR_BUFFERED_QUERY, true);

$db->setAttribute(PDO::ATTR_EMULATE_PREPARES, false);
$cursor = $db->prepare($sql, [Pdo\Tidb::ATTR_CURSOR_FETCH_SIZE => 7]);
var_dump($cursor->getAttribute(Pdo\Tidb::ATTR_CURSOR_FETCH_SIZE));
$cursor->execute();
var_dump($cursor->fetchColumn(), $cursor->fetchColumn());
$cursor->closeCursor();
$cursor->execute();
$rows = $cursor->fetchAll(PDO::FETCH_COLUMN);
var_dump(count($rows), $rows[0], $rows[49]);

$cursor->execute();
$cursor->fetch();
var_dump($db->query('SELECT 3')->fetchColumn());

var_dump($db->query('SELECT n, n * 2 AS twice FROM fetches WHERE n = 3')->fetch(PDO::FETCH_OBJ));
var_dump($db->query('SELECT n, n FROM fetches WHERE n = 4')->fetch(PDO::FETCH_NUM));
$db->setAttribute(PDO::ATTR_FETCH_TABLE_NAMES, true);
var_dump($db->query('SELECT n FROM fetches WHERE n = 5')->fetch(PDO::FETCH_ASSOC));
$db->exec('DROP TABLE fetches');
?>
--EXPECTF--
int(50)
int(1)
int(1)
int(1)
SQLSTATE[HY000]: General error: 2014 %s
int(2)
int(50)
int(50)
int(7)
int(1)
int(2)
int(50)
int(1)
int(50)
int(3)
object(stdClass)#%d (2) {
  ["n"]=>
  int(3)
  ["twice"]=>
  int(6)
}
array(2) {
  [0]=>
  int(4)
  [1]=>
  int(4)
}
array(1) {
  ["fetches.n"]=>
  int(5)
}
