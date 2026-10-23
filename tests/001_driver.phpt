--TEST--
pdo_tidb: driver registration, Pdo\Tidb class and connection attributes
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

var_dump(in_array('tidb', PDO::getAvailableDrivers(), true));
var_dump(get_parent_class(Pdo\Tidb::class));

$db = tidb_connect();
$plain = PDO::connect(tidb_dsn(), tidb_user(), tidb_password());
var_dump(get_class($plain), $plain->getAttribute(PDO::ATTR_DRIVER_NAME));

var_dump($db->getAttribute(PDO::ATTR_CLIENT_VERSION));
var_dump((bool) preg_match('/^\d+\.\d+\.\d+-TiDB-v\d+/', $db->getAttribute(PDO::ATTR_SERVER_VERSION)));
var_dump(str_starts_with($db->getAttribute(PDO::ATTR_SERVER_INFO), 'Uptime:'));
var_dump($db->getAttribute(PDO::ATTR_CONNECTION_STATUS) === tidb_host().':'.tidb_port().' via TCP/IP');
var_dump($db->getAttribute(PDO::ATTR_EMULATE_PREPARES), $db->getAttribute(PDO::ATTR_AUTOCOMMIT));
var_dump($db->getAttribute(Pdo\Tidb::ATTR_BUFFERED_QUERY), $db->getAttribute(Pdo\Tidb::ATTR_CURSOR_FETCH_SIZE));
var_dump($db->getAttribute(Pdo\Tidb::ATTR_COMPRESSION), $db->getAttribute(Pdo\Tidb::ATTR_TLS_VERSION));

$info = $db->connectionInfo();
var_dump(array_keys($info));
var_dump($info['connection_id'] === $db->connectionId(), $info['connection_id'] === (int) $db->query('SELECT CONNECTION_ID()')->fetchColumn());

$db->exec('CREATE TABLE IF NOT EXISTS ids (id BIGINT PRIMARY KEY AUTO_INCREMENT, v INT)');
$db->exec('INSERT INTO ids (v) VALUES (1)');
$id = $db->lastInsertId();
var_dump($id === (string) $db->query('SELECT MAX(id) FROM ids')->fetchColumn());
var_dump($id === (string) $db->query('SELECT LAST_INSERT_ID()')->fetchColumn());
$db->exec('DROP TABLE ids');
?>
--EXPECT--
bool(true)
string(3) "PDO"
string(8) "Pdo\Tidb"
string(4) "tidb"
string(5) "0.1.1"
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
int(0)
NULL
NULL
array(11) {
  [0]=>
  string(8) "endpoint"
  [1]=>
  string(14) "server_version"
  [2]=>
  string(13) "connection_id"
  [3]=>
  string(8) "database"
  [4]=>
  string(11) "auth_plugin"
  [5]=>
  string(11) "tls_version"
  [6]=>
  string(10) "tls_cipher"
  [7]=>
  string(11) "compression"
  [8]=>
  string(12) "capabilities"
  [9]=>
  string(6) "status"
  [10]=>
  string(15) "discovery_error"
}
bool(true)
bool(true)
bool(true)
bool(true)
