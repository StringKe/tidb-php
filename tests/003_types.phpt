--TEST--
pdo_tidb: column value decoding for text and binary protocol
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
$db->exec('DROP TABLE IF EXISTS types');
$db->exec('CREATE TABLE types (id BIGINT PRIMARY KEY AUTO_RANDOM, i INT, big BIGINT UNSIGNED, d DECIMAL(10,2), f DOUBLE, s VARCHAR(20), b VARBINARY(4), ts DATETIME(6), dt DATE, j JSON, e ENUM(\'x\',\'y\'), n INT NULL)');
$db->exec("INSERT INTO types (i, big, d, f, s, b, ts, dt, j, e, n) VALUES (-7, 18446744073709551615, 1.50, 1.25, 'a''b', x'00ff', '2026-09-29 10:00:00.123456', '2026-09-29', '{\"k\": 1}', 'y', NULL)");

$sql = 'SELECT i, big, d, f, s, HEX(b) AS b, ts, dt, j, e, n FROM types';
foreach ([true, false] as $emulate) {
    $db->setAttribute(PDO::ATTR_EMULATE_PREPARES, $emulate);
    $st = $db->prepare($sql);
    $st->execute();
    var_dump($st->fetch(PDO::FETCH_ASSOC));
    $st = $db->prepare("SELECT CAST(2026 AS YEAR), CAST('-01:02:03.5' AS TIME(3)), b'1000001', CAST(1 AS UNSIGNED)");
    $st->execute();
    var_dump($st->fetch(PDO::FETCH_NUM));
}
$db->exec('DROP TABLE types');
?>
--EXPECT--
array(11) {
  ["i"]=>
  int(-7)
  ["big"]=>
  string(20) "18446744073709551615"
  ["d"]=>
  string(4) "1.50"
  ["f"]=>
  float(1.25)
  ["s"]=>
  string(3) "a'b"
  ["b"]=>
  string(4) "00FF"
  ["ts"]=>
  string(26) "2026-09-29 10:00:00.123456"
  ["dt"]=>
  string(10) "2026-09-29"
  ["j"]=>
  string(8) "{"k": 1}"
  ["e"]=>
  string(1) "y"
  ["n"]=>
  NULL
}
array(4) {
  [0]=>
  int(2026)
  [1]=>
  string(13) "-01:02:03.500"
  [2]=>
  string(1) "A"
  [3]=>
  int(1)
}
array(11) {
  ["i"]=>
  int(-7)
  ["big"]=>
  string(20) "18446744073709551615"
  ["d"]=>
  string(4) "1.50"
  ["f"]=>
  float(1.25)
  ["s"]=>
  string(3) "a'b"
  ["b"]=>
  string(4) "00FF"
  ["ts"]=>
  string(26) "2026-09-29 10:00:00.123456"
  ["dt"]=>
  string(10) "2026-09-29"
  ["j"]=>
  string(8) "{"k": 1}"
  ["e"]=>
  string(1) "y"
  ["n"]=>
  NULL
}
array(4) {
  [0]=>
  int(2026)
  [1]=>
  string(13) "-01:02:03.500"
  [2]=>
  string(1) "A"
  [3]=>
  int(1)
}
