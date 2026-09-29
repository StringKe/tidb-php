<?php

if (!extension_loaded('pdo_tidb')) {
    die('skip pdo_tidb not loaded');
}

require __DIR__.'/config.inc';

try {
    tidb_connect();
} catch (PDOException $e) {
    die('skip TiDB server unavailable: '.$e->getMessage());
}
