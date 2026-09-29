#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include "php.h"
#include "ext/standard/info.h"
#include "ext/pdo/php_pdo.h"
#include "ext/pdo/php_pdo_driver.h"
#include "ext/pdo/php_pdo_error.h"
#include "php_pdo_tidb.h"
#include "php_pdo_tidb_int.h"
#include "pdo_tidb_arginfo.h"

static zend_class_entry *pdo_tidb_ce;

#define TIDB_METHOD_INIT() \
	pdo_dbh_t *dbh = Z_PDO_DBH_P(ZEND_THIS); \
	pdo_tidb_db_handle *H; \
	PDO_CONSTRUCT_CHECK; \
	H = dbh->driver_data; \
	PDO_DBH_CLEAR_ERR(); \
	pdo_tidb_clear(H)

#define TIDB_METHOD_FAIL() do { \
		pdo_tidb_raise(dbh, NULL); \
		PDO_HANDLE_DBH_ERR(); \
		RETURN_FALSE; \
	} while (0)

static const tidb_str tidb_none = {NULL, 0};

static void tidb_action(INTERNAL_FUNCTION_PARAMETERS, int32_t action)
{
	ZEND_PARSE_PARAMETERS_NONE();
	TIDB_METHOD_INIT();

	if (!tidb_conn_action(H->conn, action, &H->err)) {
		TIDB_METHOD_FAIL();
	}
	RETURN_TRUE;
}

PHP_METHOD(Pdo_Tidb, ping)
{
	tidb_action(INTERNAL_FUNCTION_PARAM_PASSTHRU, TIDB_ACTION_PING);
}

PHP_METHOD(Pdo_Tidb, resetConnection)
{
	tidb_action(INTERNAL_FUNCTION_PARAM_PASSTHRU, TIDB_ACTION_RESET_CONNECTION);
}

PHP_METHOD(Pdo_Tidb, killQuery)
{
	tidb_action(INTERNAL_FUNCTION_PARAM_PASSTHRU, TIDB_ACTION_KILL_QUERY);
}

PHP_METHOD(Pdo_Tidb, changeUser)
{
	zend_string *user, *password = NULL, *database = NULL;

	ZEND_PARSE_PARAMETERS_START(1, 3)
		Z_PARAM_STR(user)
		Z_PARAM_OPTIONAL
		Z_PARAM_STR_OR_NULL(password)
		Z_PARAM_STR_OR_NULL(database)
	ZEND_PARSE_PARAMETERS_END();
	TIDB_METHOD_INIT();

	if (!tidb_conn_change_user(H->conn, pdo_tidb_str(user), password ? pdo_tidb_str(password) : tidb_none, database ? pdo_tidb_str(database) : tidb_none, &H->err)) {
		TIDB_METHOD_FAIL();
	}
	RETURN_TRUE;
}

PHP_METHOD(Pdo_Tidb, selectDatabase)
{
	zend_string *database;

	ZEND_PARSE_PARAMETERS_START(1, 1)
		Z_PARAM_STR(database)
	ZEND_PARSE_PARAMETERS_END();
	TIDB_METHOD_INIT();

	if (!tidb_conn_init_db(H->conn, pdo_tidb_str(database), &H->err)) {
		TIDB_METHOD_FAIL();
	}
	RETURN_TRUE;
}

PHP_METHOD(Pdo_Tidb, statistics)
{
	tidb_owned text = {0};

	ZEND_PARSE_PARAMETERS_NONE();
	TIDB_METHOD_INIT();

	if (!tidb_conn_statistics(H->conn, &text, &H->err)) {
		TIDB_METHOD_FAIL();
	}
	RETVAL_STRINGL(text.ptr, text.len);
	tidb_owned_free(&text);
}

PHP_METHOD(Pdo_Tidb, refresh)
{
	zend_long options = 1;

	ZEND_PARSE_PARAMETERS_START(0, 1)
		Z_PARAM_OPTIONAL
		Z_PARAM_LONG(options)
	ZEND_PARSE_PARAMETERS_END();
	if (options < 0 || options > 0xFF) {
		zend_argument_value_error(1, "must be between 0 and 255");
		RETURN_THROWS();
	}
	TIDB_METHOD_INIT();

	if (!tidb_conn_refresh(H->conn, (uint8_t) options, &H->err)) {
		TIDB_METHOD_FAIL();
	}
	RETURN_TRUE;
}

PHP_METHOD(Pdo_Tidb, connectionId)
{
	ZEND_PARSE_PARAMETERS_NONE();
	TIDB_METHOD_INIT();

	RETURN_LONG((zend_long) tidb_conn_number(H->conn, TIDB_NUM_CONNECTION_ID));
}

PHP_METHOD(Pdo_Tidb, warningCount)
{
	ZEND_PARSE_PARAMETERS_NONE();
	TIDB_METHOD_INIT();

	RETURN_LONG((zend_long) tidb_conn_number(H->conn, TIDB_NUM_WARNINGS));
}

PHP_METHOD(Pdo_Tidb, serverStatus)
{
	ZEND_PARSE_PARAMETERS_NONE();
	TIDB_METHOD_INIT();

	RETURN_LONG((zend_long) tidb_conn_number(H->conn, TIDB_NUM_STATUS));
}

PHP_METHOD(Pdo_Tidb, lastInfo)
{
	tidb_owned text = {0};

	ZEND_PARSE_PARAMETERS_NONE();
	TIDB_METHOD_INIT();

	if (!tidb_conn_text(H->conn, TIDB_TEXT_LAST_INFO, &text)) {
		RETURN_NULL();
	}
	RETVAL_STRINGL(text.ptr, text.len);
	tidb_owned_free(&text);
}

PHP_METHOD(Pdo_Tidb, listFields)
{
	zend_string *table, *wildcard = NULL;
	tidb_result *result;
	tidb_column column;
	zval meta;

	ZEND_PARSE_PARAMETERS_START(1, 2)
		Z_PARAM_STR(table)
		Z_PARAM_OPTIONAL
		Z_PARAM_STR(wildcard)
	ZEND_PARSE_PARAMETERS_END();
	TIDB_METHOD_INIT();

	result = tidb_conn_field_list(H->conn, pdo_tidb_str(table), wildcard ? pdo_tidb_str(wildcard) : tidb_none, &H->err);
	if (!result) {
		TIDB_METHOD_FAIL();
	}
	array_init(return_value);
	for (size_t i = 0; tidb_result_column(result, i, &column); i++) {
		pdo_tidb_column_meta(&column, &meta);
		zend_hash_next_index_insert_new(Z_ARRVAL_P(return_value), &meta);
	}
	tidb_result_free(result);
}

PHP_METHOD(Pdo_Tidb, loadDataLocal)
{
	zend_string *sql, *data;
	uint64_t affected = 0;
	bool ok;

	ZEND_PARSE_PARAMETERS_START(2, 2)
		Z_PARAM_STR(sql)
		Z_PARAM_STR(data)
	ZEND_PARSE_PARAMETERS_END();
	TIDB_METHOD_INIT();

	tidb_conn_set_infile_data(H->conn, pdo_tidb_str(data));
	ok = tidb_conn_exec(H->conn, pdo_tidb_str(sql), &affected, &H->err);
	tidb_conn_set_infile_data(H->conn, tidb_none);
	if (!ok) {
		TIDB_METHOD_FAIL();
	}
	RETURN_LONG((zend_long) affected);
}

static void tidb_add_text(pdo_tidb_db_handle *H, zval *out, const char *key, int32_t what)
{
	tidb_owned text = {0};

	if (tidb_conn_text(H->conn, what, &text)) {
		add_assoc_stringl(out, key, text.ptr, text.len);
		tidb_owned_free(&text);
	} else {
		add_assoc_null(out, key);
	}
}

PHP_METHOD(Pdo_Tidb, connectionInfo)
{
	ZEND_PARSE_PARAMETERS_NONE();
	TIDB_METHOD_INIT();

	array_init(return_value);
	tidb_add_text(H, return_value, "endpoint", TIDB_TEXT_ENDPOINT);
	tidb_add_text(H, return_value, "server_version", TIDB_TEXT_SERVER_VERSION);
	add_assoc_long(return_value, "connection_id", (zend_long) tidb_conn_number(H->conn, TIDB_NUM_CONNECTION_ID));
	tidb_add_text(H, return_value, "database", TIDB_TEXT_DATABASE);
	tidb_add_text(H, return_value, "auth_plugin", TIDB_TEXT_AUTH_PLUGIN);
	tidb_add_text(H, return_value, "tls_version", TIDB_TEXT_TLS_VERSION);
	tidb_add_text(H, return_value, "tls_cipher", TIDB_TEXT_TLS_CIPHER);
	tidb_add_text(H, return_value, "compression", TIDB_TEXT_COMPRESSION);
	add_assoc_long(return_value, "capabilities", (zend_long) tidb_conn_number(H->conn, TIDB_NUM_CAPABILITIES));
	add_assoc_long(return_value, "status", (zend_long) tidb_conn_number(H->conn, TIDB_NUM_STATUS));
	tidb_add_text(H, return_value, "discovery_error", TIDB_TEXT_DISCOVERY_ERROR);
}

PHP_METHOD(Pdo_Tidb, errorClass)
{
	zend_long code;
	bool connection_lost = false;

	ZEND_PARSE_PARAMETERS_START(1, 2)
		Z_PARAM_LONG(code)
		Z_PARAM_OPTIONAL
		Z_PARAM_BOOL(connection_lost)
	ZEND_PARSE_PARAMETERS_END();

	RETURN_STRING(tidb_error_class_name(tidb_error_class_of(code >= 0 && code <= INT32_MAX ? (int32_t) code : -1, connection_lost)));
}

PHP_MINIT_FUNCTION(pdo_tidb)
{
	pdo_tidb_ce = register_class_Pdo_Tidb(php_pdo_get_dbh_ce());
	pdo_tidb_ce->create_object = pdo_dbh_new;

	if (php_pdo_register_driver(&pdo_tidb_driver) == FAILURE) {
		return FAILURE;
	}
	return php_pdo_register_driver_specific_ce(&pdo_tidb_driver, pdo_tidb_ce);
}

PHP_MSHUTDOWN_FUNCTION(pdo_tidb)
{
	php_pdo_unregister_driver(&pdo_tidb_driver);
	return SUCCESS;
}

PHP_MINFO_FUNCTION(pdo_tidb)
{
	php_info_print_table_start();
	php_info_print_table_row(2, "PDO Driver for TiDB", "enabled");
	php_info_print_table_row(2, "Extension version", PHP_PDO_TIDB_VERSION);
	php_info_print_table_row(2, "tidb-ffi version", tidb_driver_version());
	php_info_print_table_end();
}

static const zend_module_dep pdo_tidb_deps[] = {
	ZEND_MOD_REQUIRED("pdo")
	ZEND_MOD_END
};

zend_module_entry pdo_tidb_module_entry = {
	STANDARD_MODULE_HEADER_EX, NULL,
	pdo_tidb_deps,
	"pdo_tidb",
	NULL,
	PHP_MINIT(pdo_tidb),
	PHP_MSHUTDOWN(pdo_tidb),
	NULL,
	NULL,
	PHP_MINFO(pdo_tidb),
	PHP_PDO_TIDB_VERSION,
	STANDARD_MODULE_PROPERTIES
};

#ifdef COMPILE_DL_PDO_TIDB
ZEND_GET_MODULE(pdo_tidb)
#endif
