--TEST--
pdo_tidb: changeUser across authentication plugins
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php
require __DIR__.'/skipif.inc';
if (!is_file(tidb_certs().'/ca.pem')) {
    die('skip no test certificates in '.tidb_certs());
}
?>
--FILE--
<?php
require __DIR__.'/config.inc';

$root = tidb_connect();
$root->exec("DROP USER IF EXISTS 'cu_native'@'%', 'cu_sm3'@'%', 'cu_sha2'@'%'");
$root->exec("CREATE USER 'cu_native'@'%' IDENTIFIED WITH 'mysql_native_password' BY 'pw-native'");
$root->exec("CREATE USER 'cu_sm3'@'%' IDENTIFIED WITH 'tidb_sm3_password' BY 'pw-sm3'");
$root->exec("CREATE USER 'cu_sha2'@'%' IDENTIFIED WITH 'caching_sha2_password' BY 'pw-sha2'");
$root->exec('GRANT SELECT ON mysql.* TO cu_native, cu_sm3, cu_sha2');

$db = new Pdo\Tidb('tidb:host='.tidb_host().';port='.tidb_port().';sslmode=verify_identity;ssl_ca='.tidb_certs().'/ca.pem', tidb_user(), tidb_password(), [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION]);
foreach (['cu_native' => 'pw-native', 'cu_sm3' => 'pw-sm3', 'cu_sha2' => 'pw-sha2'] as $user => $password) {
    var_dump($db->changeUser($user, $password, 'mysql'));
    var_dump($db->query('SELECT CURRENT_USER(), DATABASE()')->fetch(PDO::FETCH_NUM), $db->getAttribute(Pdo\Tidb::ATTR_AUTH_PLUGIN));
}
tidb_error(fn () => $db->changeUser('cu_native', 'wrong'));
var_dump($db->query('SELECT CURRENT_USER()')->fetchColumn());
var_dump($db->killQuery());

$root->exec("DROP USER 'cu_native'@'%', 'cu_sm3'@'%', 'cu_sha2'@'%'");
?>
--EXPECTF--
bool(true)
array(2) {
  [0]=>
  string(11) "cu_native@%"
  [1]=>
  string(5) "mysql"
}
string(21) "mysql_native_password"
bool(true)
array(2) {
  [0]=>
  string(8) "cu_sm3@%"
  [1]=>
  string(5) "mysql"
}
string(17) "tidb_sm3_password"
bool(true)
array(2) {
  [0]=>
  string(9) "cu_sha2@%"
  [1]=>
  string(5) "mysql"
}
string(21) "caching_sha2_password"
SQLSTATE[28000]: %s: 1045 Access denied for user 'cu_native'@'%s' (using password: YES)
string(9) "cu_sha2@%"
bool(true)
