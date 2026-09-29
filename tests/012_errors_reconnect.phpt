--TEST--
pdo_tidb: errorInfo, TiDB error classes, killed connections and killQuery
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
try {
    $db->query('SELECT * FROM no_such_table');
} catch (PDOException $e) {
    var_dump($e->errorInfo);
}

foreach ([9007, 8028, 9005, 3572, 8020, 8025, 2013, 1062, 0] as $code) {
    echo $code, ' ', Pdo\Tidb::errorClass($code), "\n";
}
echo Pdo\Tidb::errorClass(1062, true), "\n";

$silent = tidb_connect([PDO::ATTR_ERRMODE => PDO::ERRMODE_SILENT]);
var_dump($silent->query('SELEC 1'), $silent->errorCode(), $silent->errorInfo()[1]);

$victim = tidb_connect();
$db->exec('KILL CONNECTION '.$victim->connectionId());
try {
    $victim->query('SELECT 1');
} catch (PDOException $e) {
    var_dump($e->errorInfo[1], Pdo\Tidb::errorClass($e->errorInfo[1]));
}
tidb_error(fn () => $victim->query('SELECT 1'));

$slow = tidb_connect();
$id = $slow->connectionId();
var_dump($db->killQuery() && $db->query('SELECT 1')->fetchColumn() === 1);
$db->exec('KILL QUERY '.$id);
var_dump($slow->query('SELECT 2')->fetchColumn());
?>
--EXPECTF--
array(3) {
  [0]=>
  string(5) "42S02"
  [1]=>
  int(1146)
  [2]=>
  string(%d) "Table 'pdo_tidb_test.no_such_table' doesn't exist"
}
9007 retry_transaction
8028 retry_transaction
9005 backoff
3572 busy
8020 retry_after_unlock
8025 fatal
2013 reconnect
1062 none
0 none
reconnect
bool(false)
string(5) "42000"
int(1064)
int(2013)
string(9) "reconnect"
SQLSTATE[08S01]: Communication link failure: 2006 %s
bool(true)
int(2)
