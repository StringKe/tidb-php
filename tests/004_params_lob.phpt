--TEST--
pdo_tidb: native and emulated parameter binding, repeated named parameters and LOB streams
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
$db->exec('DROP TABLE IF EXISTS params');
$db->exec('CREATE TABLE params (id BIGINT PRIMARY KEY AUTO_RANDOM, n INT, s VARCHAR(20), b LONGBLOB)');

foreach ([true, false] as $emulate) {
    $db->setAttribute(PDO::ATTR_EMULATE_PREPARES, $emulate);
    $db->exec('DELETE FROM params');
    $insert = $db->prepare('INSERT INTO params (n, s) VALUES (:n, :s)');
    var_dump($insert->execute([':n' => 1, ':s' => "a'b\\c"]), $insert->rowCount());
    $insert->bindValue(':n', null, PDO::PARAM_NULL);
    $insert->bindValue(':s', true, PDO::PARAM_BOOL);
    $insert->execute();

    $select = $db->prepare('SELECT s FROM params WHERE n = :n AND s = :s AND n = :n');
    $select->execute([':n' => 1, ':s' => "a'b\\c"]);
    var_dump($select->fetchAll(PDO::FETCH_COLUMN));
    var_dump($db->query('SELECT s FROM params WHERE n IS NULL')->fetchColumn());

    $positional = $db->prepare('SELECT ? + ?, ?');
    $positional->bindValue(1, 2, PDO::PARAM_INT);
    $positional->bindValue(2, 3, PDO::PARAM_INT);
    $positional->bindValue(3, '1.5');
    $positional->execute();
    var_dump($positional->fetch(PDO::FETCH_NUM));

    tidb_error(fn () => $db->prepare('SELECT ?, ?')->execute([1]));
}

$db->setAttribute(PDO::ATTR_EMULATE_PREPARES, false);
$stream = fopen('php://memory', 'r+');
fwrite($stream, str_repeat("\x01", 5 * 1024 * 1024));
rewind($stream);
$lob = $db->prepare('INSERT INTO params (n, b) VALUES (?, ?)');
$lob->bindValue(1, 2, PDO::PARAM_INT);
$lob->bindParam(2, $stream, PDO::PARAM_LOB);
$lob->execute();
var_dump($db->query('SELECT LENGTH(b) FROM params WHERE n = 2')->fetchColumn());

$long = fopen('php://memory', 'r+');
fwrite($long, str_repeat('x', 20 * 1024 * 1024));
rewind($long);
$length = $db->prepare('SELECT LENGTH(?)');
$length->bindParam(1, $long, PDO::PARAM_LOB);
$length->execute();
var_dump($length->fetchColumn());

$db->exec('DROP TABLE params');
?>
--EXPECTF--
bool(true)
int(1)
array(1) {
  [0]=>
  string(5) "a'b\c"
}
string(1) "1"
array(2) {
  [0]=>
  int(5)
  [1]=>
  string(3) "1.5"
}
SQLSTATE[HY093]: Invalid parameter number: %s
bool(true)
int(1)
array(1) {
  [0]=>
  string(5) "a'b\c"
}
string(1) "1"
array(2) {
  [0]=>
  int(5)
  [1]=>
  string(3) "1.5"
}
SQLSTATE[HY093]: Invalid parameter number: %s
int(5242880)
int(20971520)
