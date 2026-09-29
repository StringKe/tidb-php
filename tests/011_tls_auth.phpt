--TEST--
pdo_tidb: TLS modes, client certificates and the caching_sha2_password / tidb_sm3_password plugins
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

$certs = tidb_certs();
$tls = fn (string $mode, string $extra = '') => 'tidb:host='.tidb_host().';port='.tidb_port().";sslmode=$mode;ssl_ca=$certs/ca.pem".($extra === '' ? '' : ";$extra");

foreach (['preferred', 'required', 'verify_ca', 'verify_identity'] as $mode) {
    $db = new Pdo\Tidb($tls($mode), tidb_user(), tidb_password());
    var_dump($mode.' '.$db->getAttribute(Pdo\Tidb::ATTR_TLS_VERSION).' '.$db->getAttribute(PDO::ATTR_CONNECTION_STATUS));
}

$client = new Pdo\Tidb($tls('verify_ca', "ssl_cert=$certs/client.pem;ssl_key=$certs/client-key.pem"), tidb_user(), tidb_password());
var_dump($client->query("SHOW STATUS LIKE 'Ssl_cipher'")->fetch(PDO::FETCH_NUM)[1] !== '');
var_dump($client->connectionInfo()['tls_cipher'] !== null);

tidb_error(fn () => new Pdo\Tidb('tidb:host=localhost;port='.tidb_port().";sslmode=verify_identity;ssl_ca=$certs/client.pem", tidb_user(), tidb_password()));

$root = tidb_connect();
$root->exec("DROP USER IF EXISTS 'pdo_sm3'@'%', 'pdo_sha2'@'%'");
$root->exec("CREATE USER 'pdo_sm3'@'%' IDENTIFIED WITH 'tidb_sm3_password' BY 'pw-sm3'");
$root->exec("CREATE USER 'pdo_sha2'@'%' IDENTIFIED WITH 'caching_sha2_password' BY 'pw-sha2'");

foreach (['pdo_sm3' => 'pw-sm3', 'pdo_sha2' => 'pw-sha2'] as $user => $password) {
    $db = new Pdo\Tidb($tls('verify_identity', 'compression=zstd'), $user, $password);
    var_dump($db->query('SELECT CURRENT_USER()')->fetchColumn(), $db->getAttribute(Pdo\Tidb::ATTR_AUTH_PLUGIN));
    tidb_error(fn () => new Pdo\Tidb(tidb_dsn(), $user, $password));
    tidb_error(fn () => new Pdo\Tidb($tls('required'), $user, 'wrong'));
}

$root->exec("DROP USER 'pdo_sm3'@'%', 'pdo_sha2'@'%'");
?>
--EXPECTF--
string(%d) "preferred TLSv1.3 %s via TLS"
string(%d) "required TLSv1.3 %s via TLS"
string(%d) "verify_ca TLSv1.3 %s via TLS"
string(%d) "verify_identity TLSv1.3 %s via TLS"
bool(true)
bool(true)
SQLSTATE[08001] [2026] %s
string(9) "pdo_sm3@%"
string(17) "tidb_sm3_password"
SQLSTATE[28000] [2061] %s
SQLSTATE[28000] [1045] Access denied for user 'pdo_sm3'@'%s' (using password: YES)
string(10) "pdo_sha2@%"
string(21) "caching_sha2_password"
SQLSTATE[28000] [2061] %s
SQLSTATE[28000] [1045] Access denied for user 'pdo_sha2'@'%s' (using password: YES)
