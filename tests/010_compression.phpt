--TEST--
pdo_tidb: zlib and zstd protocol compression with small and large payloads and several result sets
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

foreach (['zlib' => [], 'zstd' => [Pdo\Tidb::ATTR_ZSTD_LEVEL => 7]] as $algorithm => $options) {
    $db = tidb_connect($options + [Pdo\Tidb::ATTR_COMPRESSION => $algorithm]);
    var_dump($db->getAttribute(Pdo\Tidb::ATTR_COMPRESSION), $db->connectionInfo()['compression']);
    var_dump($db->query('SELECT 1')->fetchColumn());
    var_dump(strlen($db->query("SELECT REPEAT('ab', 3000000)")->fetchColumn()));
    $db->setAttribute(PDO::ATTR_EMULATE_PREPARES, false);
    $st = $db->prepare('SELECT LENGTH(?)');
    $st->execute([str_repeat('z', 9 * 1024 * 1024)]);
    var_dump($st->fetchColumn());
    $db->setAttribute(PDO::ATTR_EMULATE_PREPARES, true);
    $st = $db->query('SELECT 1; SELECT 2; SELECT 3');
    $values = [];
    do { $values[] = $st->fetchColumn(); } while ($st->nextRowset());
    echo implode(',', $values), "\n";
}

$dsn = tidb_connect([], 'compression=zstd');
var_dump($dsn->getAttribute(Pdo\Tidb::ATTR_COMPRESSION));
tidb_error(fn () => tidb_connect([], 'compression=lz4'));
?>
--EXPECTF--
string(4) "zlib"
string(4) "zlib"
int(1)
int(6000000)
int(9437184)
1,2,3
string(4) "zstd"
string(4) "zstd"
int(1)
int(6000000)
int(9437184)
1,2,3
string(4) "zstd"
SQLSTATE[HY000] [%d] %s
