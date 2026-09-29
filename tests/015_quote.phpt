--TEST--
pdo_tidb: quote escaping, binary literals and NO_BACKSLASH_ESCAPES
--EXTENSIONS--
pdo_tidb
--SKIPIF--
<?php require __DIR__.'/skipif.inc'; ?>
--FILE--
<?php
require __DIR__.'/config.inc';

$db = tidb_connect();
$value = "a'b\\c\"\0\n\r\x1a";
var_dump($db->quote("a'b\\c"));
var_dump(bin2hex($db->query('SELECT '.$db->quote($value))->fetchColumn()) === bin2hex($value));
var_dump(bin2hex($db->quote("\0\xff", PDO::PARAM_LOB)));
var_dump(bin2hex($db->query('SELECT '.$db->quote("\0\xff", PDO::PARAM_LOB))->fetchColumn()));

$db->exec("SET SESSION sql_mode = 'NO_BACKSLASH_ESCAPES'");
var_dump($db->quote("a'b\\c"));
var_dump($db->query('SELECT '.$db->quote("a'b\\c"))->fetchColumn());
?>
--EXPECT--
string(9) "'a\'b\\c'"
bool(true)
string(24) "5f62696e617279275c30ff27"
string(4) "00ff"
string(8) "'a''b\c'"
string(5) "a'b\c"
