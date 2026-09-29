/* This is a generated file, edit the .stub.php file instead.
 * Stub hash: 3a7650ded5762a46eed9c4fd8c90c2e5dce0f0fb */

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_ping, 0, 0, _IS_BOOL, 0)
ZEND_END_ARG_INFO()

#define arginfo_class_Pdo_Tidb_resetConnection arginfo_class_Pdo_Tidb_ping

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_changeUser, 0, 1, _IS_BOOL, 0)
	ZEND_ARG_TYPE_INFO(0, user, IS_STRING, 0)
	ZEND_ARG_TYPE_INFO_WITH_DEFAULT_VALUE(0, password, IS_STRING, 1, "null")
	ZEND_ARG_TYPE_INFO_WITH_DEFAULT_VALUE(0, database, IS_STRING, 1, "null")
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_selectDatabase, 0, 1, _IS_BOOL, 0)
	ZEND_ARG_TYPE_INFO(0, database, IS_STRING, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_MASK_EX(arginfo_class_Pdo_Tidb_statistics, 0, 0, MAY_BE_STRING|MAY_BE_FALSE)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_refresh, 0, 0, _IS_BOOL, 0)
	ZEND_ARG_TYPE_INFO_WITH_DEFAULT_VALUE(0, options, IS_LONG, 0, "Pdo\\Tidb::REFRESH_GRANT")
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_connectionId, 0, 0, IS_LONG, 0)
ZEND_END_ARG_INFO()

#define arginfo_class_Pdo_Tidb_killQuery arginfo_class_Pdo_Tidb_ping

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_MASK_EX(arginfo_class_Pdo_Tidb_listFields, 0, 1, MAY_BE_ARRAY|MAY_BE_FALSE)
	ZEND_ARG_TYPE_INFO(0, table, IS_STRING, 0)
	ZEND_ARG_TYPE_INFO_WITH_DEFAULT_VALUE(0, wildcard, IS_STRING, 0, "\"\"")
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_MASK_EX(arginfo_class_Pdo_Tidb_loadDataLocal, 0, 2, MAY_BE_LONG|MAY_BE_FALSE)
	ZEND_ARG_TYPE_INFO(0, sql, IS_STRING, 0)
	ZEND_ARG_TYPE_INFO(0, data, IS_STRING, 0)
ZEND_END_ARG_INFO()

#define arginfo_class_Pdo_Tidb_warningCount arginfo_class_Pdo_Tidb_connectionId

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_lastInfo, 0, 0, IS_STRING, 1)
ZEND_END_ARG_INFO()

#define arginfo_class_Pdo_Tidb_serverStatus arginfo_class_Pdo_Tidb_connectionId

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_connectionInfo, 0, 0, IS_ARRAY, 0)
ZEND_END_ARG_INFO()

ZEND_BEGIN_ARG_WITH_RETURN_TYPE_INFO_EX(arginfo_class_Pdo_Tidb_errorClass, 0, 1, IS_STRING, 0)
	ZEND_ARG_TYPE_INFO(0, code, IS_LONG, 0)
	ZEND_ARG_TYPE_INFO_WITH_DEFAULT_VALUE(0, connectionLost, _IS_BOOL, 0, "false")
ZEND_END_ARG_INFO()

ZEND_METHOD(Pdo_Tidb, ping);
ZEND_METHOD(Pdo_Tidb, resetConnection);
ZEND_METHOD(Pdo_Tidb, changeUser);
ZEND_METHOD(Pdo_Tidb, selectDatabase);
ZEND_METHOD(Pdo_Tidb, statistics);
ZEND_METHOD(Pdo_Tidb, refresh);
ZEND_METHOD(Pdo_Tidb, connectionId);
ZEND_METHOD(Pdo_Tidb, killQuery);
ZEND_METHOD(Pdo_Tidb, listFields);
ZEND_METHOD(Pdo_Tidb, loadDataLocal);
ZEND_METHOD(Pdo_Tidb, warningCount);
ZEND_METHOD(Pdo_Tidb, lastInfo);
ZEND_METHOD(Pdo_Tidb, serverStatus);
ZEND_METHOD(Pdo_Tidb, connectionInfo);
ZEND_METHOD(Pdo_Tidb, errorClass);

static const zend_function_entry class_Pdo_Tidb_methods[] = {
	ZEND_ME(Pdo_Tidb, ping, arginfo_class_Pdo_Tidb_ping, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, resetConnection, arginfo_class_Pdo_Tidb_resetConnection, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, changeUser, arginfo_class_Pdo_Tidb_changeUser, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, selectDatabase, arginfo_class_Pdo_Tidb_selectDatabase, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, statistics, arginfo_class_Pdo_Tidb_statistics, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, refresh, arginfo_class_Pdo_Tidb_refresh, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, connectionId, arginfo_class_Pdo_Tidb_connectionId, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, killQuery, arginfo_class_Pdo_Tidb_killQuery, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, listFields, arginfo_class_Pdo_Tidb_listFields, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, loadDataLocal, arginfo_class_Pdo_Tidb_loadDataLocal, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, warningCount, arginfo_class_Pdo_Tidb_warningCount, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, lastInfo, arginfo_class_Pdo_Tidb_lastInfo, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, serverStatus, arginfo_class_Pdo_Tidb_serverStatus, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, connectionInfo, arginfo_class_Pdo_Tidb_connectionInfo, ZEND_ACC_PUBLIC)
	ZEND_ME(Pdo_Tidb, errorClass, arginfo_class_Pdo_Tidb_errorClass, ZEND_ACC_PUBLIC|ZEND_ACC_STATIC)
	ZEND_FE_END
};

static zend_class_entry *register_class_Pdo_Tidb(zend_class_entry *class_entry_PDO)
{
	zend_class_entry ce, *class_entry;

	INIT_NS_CLASS_ENTRY(ce, "Pdo", "Tidb", class_Pdo_Tidb_methods);
	class_entry = zend_register_internal_class_with_flags(&ce, class_entry_PDO, ZEND_ACC_NO_DYNAMIC_PROPERTIES|ZEND_ACC_NOT_SERIALIZABLE);

	zval const_ATTR_BUFFERED_QUERY_value;
	ZVAL_LONG(&const_ATTR_BUFFERED_QUERY_value, PDO_TIDB_ATTR_BUFFERED_QUERY);
	zend_string *const_ATTR_BUFFERED_QUERY_name = zend_string_init_interned("ATTR_BUFFERED_QUERY", sizeof("ATTR_BUFFERED_QUERY") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_BUFFERED_QUERY_name, &const_ATTR_BUFFERED_QUERY_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_BUFFERED_QUERY_name);

	zval const_ATTR_CURSOR_FETCH_SIZE_value;
	ZVAL_LONG(&const_ATTR_CURSOR_FETCH_SIZE_value, PDO_TIDB_ATTR_CURSOR_FETCH_SIZE);
	zend_string *const_ATTR_CURSOR_FETCH_SIZE_name = zend_string_init_interned("ATTR_CURSOR_FETCH_SIZE", sizeof("ATTR_CURSOR_FETCH_SIZE") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_CURSOR_FETCH_SIZE_name, &const_ATTR_CURSOR_FETCH_SIZE_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_CURSOR_FETCH_SIZE_name);

	zval const_ATTR_MULTI_STATEMENTS_value;
	ZVAL_LONG(&const_ATTR_MULTI_STATEMENTS_value, PDO_TIDB_ATTR_MULTI_STATEMENTS);
	zend_string *const_ATTR_MULTI_STATEMENTS_name = zend_string_init_interned("ATTR_MULTI_STATEMENTS", sizeof("ATTR_MULTI_STATEMENTS") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_MULTI_STATEMENTS_name, &const_ATTR_MULTI_STATEMENTS_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_MULTI_STATEMENTS_name);

	zval const_ATTR_FOUND_ROWS_value;
	ZVAL_LONG(&const_ATTR_FOUND_ROWS_value, PDO_TIDB_ATTR_FOUND_ROWS);
	zend_string *const_ATTR_FOUND_ROWS_name = zend_string_init_interned("ATTR_FOUND_ROWS", sizeof("ATTR_FOUND_ROWS") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_FOUND_ROWS_name, &const_ATTR_FOUND_ROWS_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_FOUND_ROWS_name);

	zval const_ATTR_LOCAL_INFILE_value;
	ZVAL_LONG(&const_ATTR_LOCAL_INFILE_value, PDO_TIDB_ATTR_LOCAL_INFILE);
	zend_string *const_ATTR_LOCAL_INFILE_name = zend_string_init_interned("ATTR_LOCAL_INFILE", sizeof("ATTR_LOCAL_INFILE") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_LOCAL_INFILE_name, &const_ATTR_LOCAL_INFILE_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_LOCAL_INFILE_name);

	zval const_ATTR_LOCAL_INFILE_DIRECTORY_value;
	ZVAL_LONG(&const_ATTR_LOCAL_INFILE_DIRECTORY_value, PDO_TIDB_ATTR_LOCAL_INFILE_DIRECTORY);
	zend_string *const_ATTR_LOCAL_INFILE_DIRECTORY_name = zend_string_init_interned("ATTR_LOCAL_INFILE_DIRECTORY", sizeof("ATTR_LOCAL_INFILE_DIRECTORY") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_LOCAL_INFILE_DIRECTORY_name, &const_ATTR_LOCAL_INFILE_DIRECTORY_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_LOCAL_INFILE_DIRECTORY_name);

	zval const_ATTR_INIT_COMMAND_value;
	ZVAL_LONG(&const_ATTR_INIT_COMMAND_value, PDO_TIDB_ATTR_INIT_COMMAND);
	zend_string *const_ATTR_INIT_COMMAND_name = zend_string_init_interned("ATTR_INIT_COMMAND", sizeof("ATTR_INIT_COMMAND") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_INIT_COMMAND_name, &const_ATTR_INIT_COMMAND_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_INIT_COMMAND_name);

	zval const_ATTR_SSL_MODE_value;
	ZVAL_LONG(&const_ATTR_SSL_MODE_value, PDO_TIDB_ATTR_SSL_MODE);
	zend_string *const_ATTR_SSL_MODE_name = zend_string_init_interned("ATTR_SSL_MODE", sizeof("ATTR_SSL_MODE") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_SSL_MODE_name, &const_ATTR_SSL_MODE_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_SSL_MODE_name);

	zval const_ATTR_SSL_CA_value;
	ZVAL_LONG(&const_ATTR_SSL_CA_value, PDO_TIDB_ATTR_SSL_CA);
	zend_string *const_ATTR_SSL_CA_name = zend_string_init_interned("ATTR_SSL_CA", sizeof("ATTR_SSL_CA") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_SSL_CA_name, &const_ATTR_SSL_CA_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_SSL_CA_name);

	zval const_ATTR_SSL_CAPATH_value;
	ZVAL_LONG(&const_ATTR_SSL_CAPATH_value, PDO_TIDB_ATTR_SSL_CAPATH);
	zend_string *const_ATTR_SSL_CAPATH_name = zend_string_init_interned("ATTR_SSL_CAPATH", sizeof("ATTR_SSL_CAPATH") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_SSL_CAPATH_name, &const_ATTR_SSL_CAPATH_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_SSL_CAPATH_name);

	zval const_ATTR_SSL_CERT_value;
	ZVAL_LONG(&const_ATTR_SSL_CERT_value, PDO_TIDB_ATTR_SSL_CERT);
	zend_string *const_ATTR_SSL_CERT_name = zend_string_init_interned("ATTR_SSL_CERT", sizeof("ATTR_SSL_CERT") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_SSL_CERT_name, &const_ATTR_SSL_CERT_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_SSL_CERT_name);

	zval const_ATTR_SSL_KEY_value;
	ZVAL_LONG(&const_ATTR_SSL_KEY_value, PDO_TIDB_ATTR_SSL_KEY);
	zend_string *const_ATTR_SSL_KEY_name = zend_string_init_interned("ATTR_SSL_KEY", sizeof("ATTR_SSL_KEY") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_SSL_KEY_name, &const_ATTR_SSL_KEY_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_SSL_KEY_name);

	zval const_ATTR_TLS_VERSION_value;
	ZVAL_LONG(&const_ATTR_TLS_VERSION_value, PDO_TIDB_ATTR_TLS_VERSION);
	zend_string *const_ATTR_TLS_VERSION_name = zend_string_init_interned("ATTR_TLS_VERSION", sizeof("ATTR_TLS_VERSION") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_TLS_VERSION_name, &const_ATTR_TLS_VERSION_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_TLS_VERSION_name);

	zval const_ATTR_COMPRESSION_value;
	ZVAL_LONG(&const_ATTR_COMPRESSION_value, PDO_TIDB_ATTR_COMPRESSION);
	zend_string *const_ATTR_COMPRESSION_name = zend_string_init_interned("ATTR_COMPRESSION", sizeof("ATTR_COMPRESSION") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_COMPRESSION_name, &const_ATTR_COMPRESSION_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_COMPRESSION_name);

	zval const_ATTR_ZSTD_LEVEL_value;
	ZVAL_LONG(&const_ATTR_ZSTD_LEVEL_value, PDO_TIDB_ATTR_ZSTD_LEVEL);
	zend_string *const_ATTR_ZSTD_LEVEL_name = zend_string_init_interned("ATTR_ZSTD_LEVEL", sizeof("ATTR_ZSTD_LEVEL") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_ZSTD_LEVEL_name, &const_ATTR_ZSTD_LEVEL_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_ZSTD_LEVEL_name);

	zval const_ATTR_READ_TIMEOUT_value;
	ZVAL_LONG(&const_ATTR_READ_TIMEOUT_value, PDO_TIDB_ATTR_READ_TIMEOUT);
	zend_string *const_ATTR_READ_TIMEOUT_name = zend_string_init_interned("ATTR_READ_TIMEOUT", sizeof("ATTR_READ_TIMEOUT") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_READ_TIMEOUT_name, &const_ATTR_READ_TIMEOUT_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_READ_TIMEOUT_name);

	zval const_ATTR_WRITE_TIMEOUT_value;
	ZVAL_LONG(&const_ATTR_WRITE_TIMEOUT_value, PDO_TIDB_ATTR_WRITE_TIMEOUT);
	zend_string *const_ATTR_WRITE_TIMEOUT_name = zend_string_init_interned("ATTR_WRITE_TIMEOUT", sizeof("ATTR_WRITE_TIMEOUT") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_WRITE_TIMEOUT_name, &const_ATTR_WRITE_TIMEOUT_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_WRITE_TIMEOUT_name);

	zval const_ATTR_DISCOVER_value;
	ZVAL_LONG(&const_ATTR_DISCOVER_value, PDO_TIDB_ATTR_DISCOVER);
	zend_string *const_ATTR_DISCOVER_name = zend_string_init_interned("ATTR_DISCOVER", sizeof("ATTR_DISCOVER") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_DISCOVER_name, &const_ATTR_DISCOVER_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_DISCOVER_name);

	zval const_ATTR_MAX_LIFETIME_value;
	ZVAL_LONG(&const_ATTR_MAX_LIFETIME_value, PDO_TIDB_ATTR_MAX_LIFETIME);
	zend_string *const_ATTR_MAX_LIFETIME_name = zend_string_init_interned("ATTR_MAX_LIFETIME", sizeof("ATTR_MAX_LIFETIME") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_MAX_LIFETIME_name, &const_ATTR_MAX_LIFETIME_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_MAX_LIFETIME_name);

	zval const_ATTR_IDLE_PING_value;
	ZVAL_LONG(&const_ATTR_IDLE_PING_value, PDO_TIDB_ATTR_IDLE_PING);
	zend_string *const_ATTR_IDLE_PING_name = zend_string_init_interned("ATTR_IDLE_PING", sizeof("ATTR_IDLE_PING") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_IDLE_PING_name, &const_ATTR_IDLE_PING_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_IDLE_PING_name);

	zval const_ATTR_AUTH_PLUGIN_value;
	ZVAL_LONG(&const_ATTR_AUTH_PLUGIN_value, PDO_TIDB_ATTR_AUTH_PLUGIN);
	zend_string *const_ATTR_AUTH_PLUGIN_name = zend_string_init_interned("ATTR_AUTH_PLUGIN", sizeof("ATTR_AUTH_PLUGIN") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_ATTR_AUTH_PLUGIN_name, &const_ATTR_AUTH_PLUGIN_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_ATTR_AUTH_PLUGIN_name);

	zval const_REFRESH_GRANT_value;
	ZVAL_LONG(&const_REFRESH_GRANT_value, 1);
	zend_string *const_REFRESH_GRANT_name = zend_string_init_interned("REFRESH_GRANT", sizeof("REFRESH_GRANT") - 1, 1);
	zend_declare_typed_class_constant(class_entry, const_REFRESH_GRANT_name, &const_REFRESH_GRANT_value, ZEND_ACC_PUBLIC, NULL, (zend_type) ZEND_TYPE_INIT_MASK(MAY_BE_LONG));
	zend_string_release(const_REFRESH_GRANT_name);

	return class_entry;
}
