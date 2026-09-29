--TEST--
pdo_tidb: getColumnMeta and listFields
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
$db->exec('DROP TABLE IF EXISTS meta');
$db->exec('CREATE TABLE meta (id BIGINT PRIMARY KEY AUTO_RANDOM, name VARCHAR(20) NOT NULL, amount DECIMAL(36,18), payload BLOB, UNIQUE KEY meta_name_unique (name))');

$st = $db->query('SELECT id, name AS label, amount, payload FROM meta');
foreach (range(0, 3) as $i) {
    $meta = $st->getColumnMeta($i);
    echo $meta['name'], ' ', $meta['native_type'], ' ', $meta['pdo_type'], ' ', $meta['len'], ' ', $meta['precision'], ' ', $meta['table'], ' ', $meta['tidb:org_name'], ' ', $meta['tidb:schema'], ' [', implode(',', $meta['flags']), "]\n";
}
var_dump($st->getColumnMeta(4));

foreach ($db->listFields('meta', 'na%') as $field) {
    echo $field['name'], ' ', $field['native_type'], "\n";
}
var_dump(count($db->listFields('meta')));
tidb_error(fn () => $db->listFields('no_such_table'));
$db->exec('DROP TABLE meta');
?>
--EXPECTF--
id LONGLONG 1 20 0 meta id pdo_tidb_test [not_null,primary_key]
label VAR_STRING 2 80 0 meta name pdo_tidb_test [not_null,unique_key]
amount NEWDECIMAL 2 %d 18 meta amount pdo_tidb_test []
payload BLOB 3 65535 0 meta payload pdo_tidb_test [binary]
bool(false)
name VAR_STRING
int(4)
SQLSTATE[42S02]: %s
