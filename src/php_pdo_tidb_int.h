#ifndef PHP_PDO_TIDB_INT_H
#define PHP_PDO_TIDB_INT_H

#include "tidb_ffi.h"

typedef struct {
	tidb_conn *conn;
	tidb_error err;
	bool emulate_prepares;
	bool buffered;
	bool fetch_table_names;
	uint32_t fetch_size;
} pdo_tidb_db_handle;

typedef struct {
	pdo_tidb_db_handle *H;
	tidb_stmt *stmt;
	tidb_result *result;
	bool buffered;
	uint32_t fetch_size;
} pdo_tidb_stmt;

enum {
	PDO_TIDB_ATTR_BUFFERED_QUERY = PDO_ATTR_DRIVER_SPECIFIC,
	PDO_TIDB_ATTR_CURSOR_FETCH_SIZE,
	PDO_TIDB_ATTR_MULTI_STATEMENTS,
	PDO_TIDB_ATTR_FOUND_ROWS,
	PDO_TIDB_ATTR_LOCAL_INFILE,
	PDO_TIDB_ATTR_LOCAL_INFILE_DIRECTORY,
	PDO_TIDB_ATTR_INIT_COMMAND,
	PDO_TIDB_ATTR_SSL_MODE,
	PDO_TIDB_ATTR_SSL_CA,
	PDO_TIDB_ATTR_SSL_CAPATH,
	PDO_TIDB_ATTR_SSL_CERT,
	PDO_TIDB_ATTR_SSL_KEY,
	PDO_TIDB_ATTR_TLS_VERSION,
	PDO_TIDB_ATTR_COMPRESSION,
	PDO_TIDB_ATTR_ZSTD_LEVEL,
	PDO_TIDB_ATTR_READ_TIMEOUT,
	PDO_TIDB_ATTR_WRITE_TIMEOUT,
	PDO_TIDB_ATTR_DISCOVER,
	PDO_TIDB_ATTR_MAX_LIFETIME,
	PDO_TIDB_ATTR_IDLE_PING,
	PDO_TIDB_ATTR_AUTH_PLUGIN,
};

extern const pdo_driver_t pdo_tidb_driver;
extern const struct pdo_stmt_methods tidb_stmt_methods;

/* Copies the pending tidb_error into the PDO error slot of stmt (or dbh when stmt is NULL). */
void pdo_tidb_raise(pdo_dbh_t *dbh, pdo_stmt_t *stmt);
/* Column metadata in the shape PDOStatement::getColumnMeta() returns. */
void pdo_tidb_column_meta(const tidb_column *c, zval *out);

static inline void pdo_tidb_clear(pdo_tidb_db_handle *H)
{
	if (H->err.message.ptr || H->err.sqlstate[0]) {
		tidb_error_clear(&H->err);
	}
}

static inline tidb_str pdo_tidb_str(const zend_string *s)
{
	tidb_str out = { s ? ZSTR_VAL(s) : NULL, s ? ZSTR_LEN(s) : 0 };
	return out;
}

static inline tidb_str pdo_tidb_cstr(const char *s, size_t len)
{
	tidb_str out = { s, len };
	return out;
}

static inline void pdo_tidb_cell_zval(const tidb_cell *c, zval *out)
{
	switch (c->kind) {
		case TIDB_CELL_INT:
			ZVAL_LONG(out, (zend_long) c->int_value);
			break;
		case TIDB_CELL_FLOAT:
			ZVAL_DOUBLE(out, c->float_value);
			break;
		case TIDB_CELL_BYTES:
			ZVAL_STRINGL_FAST(out, c->data.ptr, c->data.len);
			break;
		default:
			ZVAL_NULL(out);
	}
}

#endif
