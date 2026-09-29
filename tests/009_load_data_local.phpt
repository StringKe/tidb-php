--TEST--
pdo_tidb: LOAD DATA LOCAL from memory and from files under local_infile_directory
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$dir = sys_get_temp_dir().'/pdo_tidb_infile_'.getmypid();
@mkdir($dir);
file_put_contents($dir.'/rows.csv', "3,c\n4,d\n5,e\n");

$db = tidb_connect([Pdo\Tidb::ATTR_LOCAL_INFILE => true, Pdo\Tidb::ATTR_LOCAL_INFILE_DIRECTORY => $dir]);
$db->exec('DROP TABLE IF EXISTS infile_rows');
$db->exec('CREATE TABLE infile_rows (id BIGINT PRIMARY KEY AUTO_RANDOM, n INT, s VARCHAR(8))');

var_dump($db->loadDataLocal("LOAD DATA LOCAL INFILE 'memory.csv' INTO TABLE infile_rows FIELDS TERMINATED BY ',' (n, s)", "1,a\n2,b\n"));
var_dump($db->exec("LOAD DATA LOCAL INFILE '$dir/rows.csv' INTO TABLE infile_rows FIELDS TERMINATED BY ',' (n, s)"));
var_dump($db->query('SELECT GROUP_CONCAT(s ORDER BY n) FROM infile_rows')->fetchColumn());
tidb_error(fn () => $db->exec("LOAD DATA LOCAL INFILE '/etc/hosts' INTO TABLE infile_rows (s)"));

$off = tidb_connect();
tidb_error(fn () => $off->exec("LOAD DATA LOCAL INFILE '$dir/rows.csv' INTO TABLE infile_rows FIELDS TERMINATED BY ',' (n, s)"));
var_dump($off->query('SELECT COUNT(*) FROM infile_rows')->fetchColumn());

$db->exec('DROP TABLE infile_rows');
unlink($dir.'/rows.csv');
rmdir($dir);
?>
--EXPECTF--
int(2)
int(3)
string(9) "a,b,c,d,e"
SQLSTATE[HY000]: General error: 2068 %s
SQLSTATE[HY000]: General error: 2068 %s
int(5)
