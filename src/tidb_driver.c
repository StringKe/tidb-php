#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include "php.h"
#include "ext/pdo/php_pdo.h"
#include "ext/pdo/php_pdo_driver.h"
#include "ext/pdo/php_pdo_error.h"
#include "php_pdo_tidb.h"
#include "php_pdo_tidb_int.h"

#define TIDB_ER_UNSUPPORTED_PS 1295

void pdo_tidb_raise(pdo_dbh_t *dbh, pdo_stmt_t *stmt)
{
	pdo_tidb_db_handle *H = dbh->driver_data;
	pdo_error_type *slot = stmt ? &stmt->error_code : &dbh->error_code;
	const char *state = H->err.sqlstate[0] ? H->err.sqlstate : "HY000";

	memcpy(*slot, state, 5);
	(*slot)[5] = '\0';

	if (!dbh->methods) {
		pdo_throw_exception((unsigned int) H->err.code, H->err.message.ptr ? H->err.message.ptr : "unknown error", &dbh->error_code);
	}
}

static void tidb_handle_closer(pdo_dbh_t *dbh)
{
	pdo_tidb_db_handle *H = dbh->driver_data;

	if (!H) {
		return;
	}
	if (H->conn) {
		tidb_conn_free(H->conn);
		H->conn = NULL;
	}
	tidb_error_clear(&H->err);
	pefree(H, dbh->is_persistent);
	dbh->driver_data = NULL;
}

static bool tidb_handle_preparer(pdo_dbh_t *dbh, zend_string *sql, pdo_stmt_t *stmt, zval *driver_options)
{
	pdo_tidb_db_handle *H = dbh->driver_data;
	pdo_tidb_stmt *S = ecalloc(1, sizeof(*S));
	zend_string *nsql = NULL;
	int ret;

	S->H = H;
	S->buffered = pdo_attr_lval(driver_options, PDO_TIDB_ATTR_BUFFERED_QUERY, H->buffered) != 0;
	S->fetch_size = (uint32_t) pdo_attr_lval(driver_options, PDO_TIDB_ATTR_CURSOR_FETCH_SIZE, H->fetch_size);
	stmt->driver_data = S;
	stmt->methods = &tidb_stmt_methods;

	if (pdo_attr_lval(driver_options, PDO_ATTR_EMULATE_PREPARES, H->emulate_prepares)) {
		stmt->supports_placeholders = PDO_PLACEHOLDER_NONE;
		return true;
	}

	stmt->supports_placeholders = PDO_PLACEHOLDER_POSITIONAL;
	ret = pdo_parse_params(stmt, sql, &nsql);
	if (ret == -1) {
		strcpy(dbh->error_code, stmt->error_code);
		return false;
	}
	if (stmt->bound_param_map) {
		/* Positions resolve through bound_param_map at execute time, so one :name may appear several times. */
		stmt->named_rewrite_template = "?";
	}

	pdo_tidb_clear(H);
	S->stmt = tidb_conn_prepare(H->conn, pdo_tidb_str(ret == 1 ? nsql : sql), &H->err);
	if (nsql) {
		zend_string_release_ex(nsql, 0);
	}
	if (!S->stmt) {
		if (H->err.code == TIDB_ER_UNSUPPORTED_PS) {
			pdo_tidb_clear(H);
			stmt->supports_placeholders = PDO_PLACEHOLDER_NONE;
			stmt->named_rewrite_template = NULL;
			return true;
		}
		pdo_tidb_raise(dbh, NULL);
		return false;
	}
	return true;
}

static zend_long tidb_handle_doer(pdo_dbh_t *dbh, const zend_string *sql)
{
	pdo_tidb_db_handle *H = dbh->driver_data;
	uint64_t affected = 0;

	pdo_tidb_clear(H);
	if (!tidb_conn_exec(H->conn, pdo_tidb_str(sql), &affected, &H->err)) {
		pdo_tidb_raise(dbh, NULL);
		return -1;
	}
	return (zend_long) affected;
}

static zend_string *tidb_handle_quoter(pdo_dbh_t *dbh, const zend_string *unquoted, enum pdo_param_type paramtype)
{
	pdo_tidb_db_handle *H = dbh->driver_data;
	tidb_owned quoted = {0};
	zend_string *out;

	tidb_conn_quote(H->conn, pdo_tidb_str(unquoted), PDO_PARAM_TYPE(paramtype) == PDO_PARAM_LOB, &quoted);
	out = zend_string_init(quoted.ptr, quoted.len, 0);
	tidb_owned_free(&quoted);
	return out;
}

static bool tidb_exec_simple(pdo_dbh_t *dbh, const char *sql, size_t len)
{
	pdo_tidb_db_handle *H = dbh->driver_data;

	pdo_tidb_clear(H);
	if (!tidb_conn_exec(H->conn, pdo_tidb_cstr(sql, len), NULL, &H->err)) {
		pdo_tidb_raise(dbh, NULL);
		return false;
	}
	return true;
}

static bool tidb_handle_begin(pdo_dbh_t *dbh)
{
	return tidb_exec_simple(dbh, ZEND_STRL("BEGIN"));
}

static bool tidb_handle_commit(pdo_dbh_t *dbh)
{
	return tidb_exec_simple(dbh, ZEND_STRL("COMMIT"));
}

static bool tidb_handle_rollback(pdo_dbh_t *dbh)
{
	return tidb_exec_simple(dbh, ZEND_STRL("ROLLBACK"));
}

static bool tidb_handle_in_transaction(pdo_dbh_t *dbh)
{
	pdo_tidb_db_handle *H = dbh->driver_data;

	return H && H->conn && tidb_conn_number(H->conn, TIDB_NUM_IN_TRANSACTION) != 0;
}

static bool tidb_set_autocommit(pdo_dbh_t *dbh, bool on)
{
	if (on) {
		return tidb_exec_simple(dbh, ZEND_STRL("SET autocommit = 1"));
	}
	return tidb_exec_simple(dbh, ZEND_STRL("SET autocommit = 0"));
}

static bool tidb_handle_set_attr(pdo_dbh_t *dbh, zend_long attr, zval *val)
{
	pdo_tidb_db_handle *H = dbh->driver_data;
	zend_long lval;
	bool bval;

	switch (attr) {
		case PDO_ATTR_EMULATE_PREPARES:
			if (!pdo_get_bool_param(&bval, val)) {
				return false;
			}
			H->emulate_prepares = bval;
			return true;
		case PDO_ATTR_AUTOCOMMIT:
			if (!pdo_get_bool_param(&bval, val)) {
				return false;
			}
			if (!tidb_set_autocommit(dbh, bval)) {
				return false;
			}
			dbh->auto_commit = bval;
			return true;
		case PDO_ATTR_FETCH_TABLE_NAMES:
			if (!pdo_get_bool_param(&bval, val)) {
				return false;
			}
			H->fetch_table_names = bval;
			return true;
		case PDO_TIDB_ATTR_BUFFERED_QUERY:
			if (!pdo_get_bool_param(&bval, val)) {
				return false;
			}
			H->buffered = bval;
			return true;
		case PDO_TIDB_ATTR_CURSOR_FETCH_SIZE:
			if (!pdo_get_long_param(&lval, val)) {
				return false;
			}
			if (lval < 0 || lval > UINT32_MAX) {
				zend_value_error("Pdo\\Tidb::ATTR_CURSOR_FETCH_SIZE must be between 0 and %u", UINT32_MAX);
				return false;
			}
			H->fetch_size = (uint32_t) lval;
			return true;
		case PDO_TIDB_ATTR_MULTI_STATEMENTS:
			if (!pdo_get_bool_param(&bval, val)) {
				return false;
			}
			pdo_tidb_clear(H);
			if (!tidb_conn_action(H->conn, bval ? TIDB_ACTION_MULTI_STATEMENTS_ON : TIDB_ACTION_MULTI_STATEMENTS_OFF, &H->err)) {
				pdo_tidb_raise(dbh, NULL);
				return false;
			}
			return true;
		default:
			return false;
	}
}

static zend_string *tidb_handle_last_id(pdo_dbh_t *dbh, const zend_string *name)
{
	pdo_tidb_db_handle *H = dbh->driver_data;

	return zend_u64_to_str(tidb_conn_number(H->conn, TIDB_NUM_LAST_INSERT_ID));
}

static bool tidb_text_zval(pdo_tidb_db_handle *H, int32_t what, zval *val)
{
	tidb_owned text = {0};

	if (!tidb_conn_text(H->conn, what, &text)) {
		return false;
	}
	ZVAL_STRINGL(val, text.ptr, text.len);
	tidb_owned_free(&text);
	return true;
}

static int tidb_handle_get_attr(pdo_dbh_t *dbh, zend_long attr, zval *val)
{
	pdo_tidb_db_handle *H = dbh->driver_data;
	tidb_owned text = {0};

	switch (attr) {
		case PDO_ATTR_EMULATE_PREPARES:
			ZVAL_BOOL(val, H->emulate_prepares);
			return 1;
		case PDO_ATTR_AUTOCOMMIT:
			ZVAL_BOOL(val, (tidb_conn_number(H->conn, TIDB_NUM_STATUS) & TIDB_STATUS_AUTOCOMMIT) != 0);
			return 1;
		case PDO_ATTR_FETCH_TABLE_NAMES:
			ZVAL_BOOL(val, H->fetch_table_names);
			return 1;
		case PDO_ATTR_CLIENT_VERSION:
			ZVAL_STRING(val, tidb_driver_version());
			return 1;
		case PDO_ATTR_SERVER_VERSION:
			if (!tidb_text_zval(H, TIDB_TEXT_SERVER_VERSION, val)) {
				ZVAL_NULL(val);
			}
			return 1;
		case PDO_ATTR_SERVER_INFO:
			pdo_tidb_clear(H);
			if (!tidb_conn_statistics(H->conn, &text, &H->err)) {
				pdo_tidb_raise(dbh, NULL);
				return -1;
			}
			ZVAL_STRINGL(val, text.ptr, text.len);
			tidb_owned_free(&text);
			return 1;
		case PDO_ATTR_CONNECTION_STATUS:
			if (!tidb_conn_text(H->conn, TIDB_TEXT_ENDPOINT, &text)) {
				ZVAL_NULL(val);
				return 1;
			}
			ZVAL_STR(val, zend_strpprintf(0, "%s via %s", text.ptr, tidb_conn_text(H->conn, TIDB_TEXT_TLS_VERSION, NULL) ? "TLS" : "TCP/IP"));
			tidb_owned_free(&text);
			return 1;
		case PDO_TIDB_ATTR_BUFFERED_QUERY:
			ZVAL_BOOL(val, H->buffered);
			return 1;
		case PDO_TIDB_ATTR_CURSOR_FETCH_SIZE:
			ZVAL_LONG(val, (zend_long) H->fetch_size);
			return 1;
		case PDO_TIDB_ATTR_COMPRESSION:
			if (!tidb_text_zval(H, TIDB_TEXT_COMPRESSION, val)) {
				ZVAL_NULL(val);
			}
			return 1;
		case PDO_TIDB_ATTR_TLS_VERSION:
			if (!tidb_text_zval(H, TIDB_TEXT_TLS_VERSION, val)) {
				ZVAL_NULL(val);
			}
			return 1;
		case PDO_TIDB_ATTR_AUTH_PLUGIN:
			tidb_text_zval(H, TIDB_TEXT_AUTH_PLUGIN, val);
			return 1;
		default:
			return 0;
	}
}

static void tidb_handle_fetch_error(pdo_dbh_t *dbh, pdo_stmt_t *stmt, zval *info)
{
	pdo_tidb_db_handle *H = dbh->driver_data;

	if (H && H->err.message.ptr) {
		add_next_index_long(info, (zend_long) H->err.code);
		add_next_index_stringl(info, H->err.message.ptr, H->err.message.len);
	}
}

static zend_result tidb_handle_check_liveness(pdo_dbh_t *dbh)
{
	pdo_tidb_db_handle *H = dbh->driver_data;

	return H && H->conn && tidb_conn_alive(H->conn) ? SUCCESS : FAILURE;
}

static const struct pdo_dbh_methods tidb_methods = {
	tidb_handle_closer,
	tidb_handle_preparer,
	tidb_handle_doer,
	tidb_handle_quoter,
	tidb_handle_begin,
	tidb_handle_commit,
	tidb_handle_rollback,
	tidb_handle_set_attr,
	tidb_handle_last_id,
	tidb_handle_fetch_error,
	tidb_handle_get_attr,
	tidb_handle_check_liveness,
	NULL, /* get_driver_methods: extensions live on Pdo\Tidb */
	NULL, /* persistent_shutdown */
	tidb_handle_in_transaction,
	NULL, /* get_gc */
	NULL  /* scanner */
};

/* Connection attributes passed to the constructor become DSN keys; they win over the DSN string. */
static const struct {
	zend_long attr;
	const char *key;
} tidb_connect_attrs[] = {
	{PDO_ATTR_TIMEOUT, "connect_timeout"},
	{PDO_TIDB_ATTR_MULTI_STATEMENTS, "multi_statements"},
	{PDO_TIDB_ATTR_FOUND_ROWS, "found_rows"},
	{PDO_TIDB_ATTR_LOCAL_INFILE, "local_infile"},
	{PDO_TIDB_ATTR_LOCAL_INFILE_DIRECTORY, "local_infile_directory"},
	{PDO_TIDB_ATTR_INIT_COMMAND, "init_command"},
	{PDO_TIDB_ATTR_SSL_MODE, "sslmode"},
	{PDO_TIDB_ATTR_SSL_CA, "ssl_ca"},
	{PDO_TIDB_ATTR_SSL_CAPATH, "ssl_capath"},
	{PDO_TIDB_ATTR_SSL_CERT, "ssl_cert"},
	{PDO_TIDB_ATTR_SSL_KEY, "ssl_key"},
	{PDO_TIDB_ATTR_TLS_VERSION, "tls_version"},
	{PDO_TIDB_ATTR_COMPRESSION, "compression"},
	{PDO_TIDB_ATTR_ZSTD_LEVEL, "zstd_level"},
	{PDO_TIDB_ATTR_READ_TIMEOUT, "read_timeout"},
	{PDO_TIDB_ATTR_WRITE_TIMEOUT, "write_timeout"},
	{PDO_TIDB_ATTR_DISCOVER, "discover"},
	{PDO_TIDB_ATTR_MAX_LIFETIME, "max_lifetime"},
	{PDO_TIDB_ATTR_IDLE_PING, "idle_ping"},
	{PDO_TIDB_ATTR_AUTH_PLUGIN, "auth_plugin"},
};

#define TIDB_CONNECT_ATTR_COUNT (sizeof(tidb_connect_attrs) / sizeof(tidb_connect_attrs[0]))

static int pdo_tidb_handle_factory(pdo_dbh_t *dbh, zval *driver_options)
{
	pdo_tidb_db_handle *H = pecalloc(1, sizeof(*H), dbh->is_persistent);
	tidb_str user = {dbh->username, dbh->username ? strlen(dbh->username) : 0};
	tidb_str password = {dbh->password, dbh->password ? strlen(dbh->password) : 0};
	tidb_option options[TIDB_CONNECT_ATTR_COUNT];
	zend_string *values[TIDB_CONNECT_ATTR_COUNT];
	size_t count = 0;
	int ok = 0;

	dbh->driver_data = H;
	dbh->alloc_own_columns = 1;
	dbh->max_escaped_char_length = 2;
	H->emulate_prepares = true;
	H->buffered = true;

	if (driver_options) {
		HashTable *ht = Z_ARRVAL_P(driver_options);

		for (size_t i = 0; i < TIDB_CONNECT_ATTR_COUNT; i++) {
			zval *v = zend_hash_index_find(ht, tidb_connect_attrs[i].attr);

			if (!v || Z_TYPE_P(v) == IS_NULL) {
				continue;
			}
			values[count] = zval_try_get_string(v);
			if (!values[count]) {
				goto cleanup;
			}
			options[count].key = pdo_tidb_cstr(tidb_connect_attrs[i].key, strlen(tidb_connect_attrs[i].key));
			options[count].value = pdo_tidb_str(values[count]);
			count++;
		}
		H->emulate_prepares = pdo_attr_lval(driver_options, PDO_ATTR_EMULATE_PREPARES, 1) != 0;
		H->buffered = pdo_attr_lval(driver_options, PDO_TIDB_ATTR_BUFFERED_QUERY, 1) != 0;
		H->fetch_size = (uint32_t) pdo_attr_lval(driver_options, PDO_TIDB_ATTR_CURSOR_FETCH_SIZE, 0);
		H->fetch_table_names = pdo_attr_lval(driver_options, PDO_ATTR_FETCH_TABLE_NAMES, 0) != 0;
	}

	H->conn = tidb_connect(pdo_tidb_cstr(dbh->data_source, dbh->data_source_len), user, password, options, count, &H->err);
	if (!H->conn) {
		pdo_tidb_raise(dbh, NULL);
		goto cleanup;
	}
	ok = 1;

cleanup:
	for (size_t i = 0; i < count; i++) {
		zend_string_release(values[i]);
	}
	if (ok && driver_options && zend_hash_index_exists(Z_ARRVAL_P(driver_options), PDO_ATTR_AUTOCOMMIT)) {
		dbh->auto_commit = pdo_attr_lval(driver_options, PDO_ATTR_AUTOCOMMIT, 1) != 0;
		if (!dbh->auto_commit && !tidb_set_autocommit(dbh, false)) {
			ok = 0;
		}
	}
	/* Set last: while methods is NULL, pdo_tidb_raise() throws, which is how a failed constructor reports. */
	dbh->methods = &tidb_methods;
	if (!ok) {
		tidb_handle_closer(dbh);
	}
	return ok;
}

const pdo_driver_t pdo_tidb_driver = {
	PDO_DRIVER_HEADER(tidb),
	pdo_tidb_handle_factory
};
