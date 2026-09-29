#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include "php.h"
#include "ext/pdo/php_pdo.h"
#include "ext/pdo/php_pdo_driver.h"
#include "ext/pdo/php_pdo_error.h"
#include "php_pdo_tidb_int.h"

#define TIDB_STACK_PARAMS 16

static bool tidb_stmt_release_result(pdo_tidb_stmt *S)
{
	bool ok = true;

	if (S->result) {
		if (S->H->conn) {
			ok = tidb_result_close(S->H->conn, S->result, &S->H->err);
		}
		tidb_result_free(S->result);
		S->result = NULL;
	}
	return ok;
}

static int tidb_stmt_dtor(pdo_stmt_t *stmt)
{
	pdo_tidb_stmt *S = stmt->driver_data;

	tidb_stmt_release_result(S);
	pdo_tidb_clear(S->H);
	if (S->stmt) {
		tidb_stmt_free(S->H->conn, S->stmt);
	}
	efree(S);
	stmt->driver_data = NULL;
	return 1;
}

static struct pdo_bound_param_data *tidb_stmt_param_at(const pdo_stmt_t *stmt, size_t position)
{
	struct pdo_bound_param_data *param = NULL;
	zend_string *name;

	if (!stmt->bound_params) {
		return NULL;
	}
	if (stmt->bound_param_map && (name = zend_hash_index_find_ptr(stmt->bound_param_map, position)) != NULL) {
		param = zend_hash_find_ptr(stmt->bound_params, name);
	}
	return param ? param : zend_hash_index_find_ptr(stmt->bound_params, position);
}

/* Strings are borrowed from the bound zval; streams and non-string scalars produce a temporary in *temp. */
static bool tidb_stmt_fill_param(struct pdo_bound_param_data *param, tidb_param *out, zend_string **temp)
{
	zval *value = &param->parameter;
	bool lob = PDO_PARAM_TYPE(param->param_type) == PDO_PARAM_LOB;
	zend_string *str;

	ZVAL_DEREF(value);
	memset(out, 0, sizeof(*out));
	*temp = NULL;

	if (lob && Z_TYPE_P(value) == IS_RESOURCE) {
		php_stream *stream;

		php_stream_from_zval_no_verify(stream, value);
		if (!stream) {
			pdo_raise_impl_error(NULL, NULL, "HY105", "Expected a stream resource");
			return false;
		}
		*temp = php_stream_copy_to_mem(stream, PHP_STREAM_COPY_ALL, 0);
		if (!*temp) {
			*temp = ZSTR_EMPTY_ALLOC();
		}
		out->kind = TIDB_PARAM_LONG_DATA;
		out->data = pdo_tidb_str(*temp);
		return true;
	}
	switch (Z_TYPE_P(value)) {
		case IS_NULL:
			out->kind = TIDB_PARAM_NULL;
			return true;
		case IS_FALSE:
		case IS_TRUE:
			out->kind = TIDB_PARAM_INT;
			out->int_value = Z_TYPE_P(value) == IS_TRUE;
			return true;
		case IS_LONG:
			out->kind = TIDB_PARAM_INT;
			out->int_value = Z_LVAL_P(value);
			return true;
		case IS_DOUBLE:
			out->kind = TIDB_PARAM_FLOAT;
			out->float_value = Z_DVAL_P(value);
			return true;
		case IS_STRING:
			str = Z_STR_P(value);
			break;
		default:
			str = zval_try_get_string(value);
			if (!str) {
				return false;
			}
			*temp = str;
	}
	out->kind = lob ? TIDB_PARAM_LONG_DATA : TIDB_PARAM_BYTES;
	out->data = pdo_tidb_str(str);
	return true;
}

static int32_t tidb_stmt_mode(const pdo_tidb_stmt *S, bool native)
{
	if (native && S->fetch_size > 0) {
		return TIDB_MODE_CURSOR;
	}
	return S->buffered ? TIDB_MODE_BUFFERED : TIDB_MODE_STREAMING;
}

static tidb_result *tidb_stmt_run_native(pdo_stmt_t *stmt, pdo_tidb_stmt *S)
{
	tidb_param stack_params[TIDB_STACK_PARAMS];
	zend_string *stack_temps[TIDB_STACK_PARAMS];
	size_t count = tidb_stmt_param_count(S->stmt);
	tidb_param *params = count > TIDB_STACK_PARAMS ? safe_emalloc(count, sizeof(tidb_param), 0) : stack_params;
	zend_string **temps = count > TIDB_STACK_PARAMS ? safe_emalloc(count, sizeof(zend_string *), 0) : stack_temps;
	tidb_result *result = NULL;
	size_t filled = 0;

	for (; filled < count; filled++) {
		struct pdo_bound_param_data *param = tidb_stmt_param_at(stmt, filled);

		if (!param) {
			char message[64];
			snprintf(message, sizeof(message), "parameter %zu was not bound", filled + 1);
			pdo_raise_impl_error(stmt->dbh, stmt, "HY093", message);
			goto done;
		}
		if (!tidb_stmt_fill_param(param, &params[filled], &temps[filled])) {
			goto done;
		}
	}

	result = tidb_stmt_execute(S->H->conn, S->stmt, params, count, tidb_stmt_mode(S, true), S->fetch_size, &S->H->err);
	if (!result) {
		pdo_tidb_raise(stmt->dbh, stmt);
	}

done:
	for (size_t i = 0; i < filled; i++) {
		if (temps[i]) {
			zend_string_release_ex(temps[i], 0);
		}
	}
	if (params != stack_params) {
		efree(params);
		efree(temps);
	}
	return result;
}

/* Sets the column count and row count PDO reports for the current result set. */
static void tidb_stmt_adopt_result(pdo_stmt_t *stmt, pdo_tidb_stmt *S)
{
	tidb_result_info info;
	size_t columns = tidb_result_column_count(S->result);

	php_pdo_stmt_set_column_count(stmt, (int) columns);
	tidb_result_get_info(S->result, &info);
	if (columns == 0) {
		stmt->row_count = (zend_long) info.affected_rows;
	} else {
		stmt->row_count = info.has_row_count ? (zend_long) info.row_count : 0;
	}
}

static int tidb_stmt_execute_cb(pdo_stmt_t *stmt)
{
	pdo_tidb_stmt *S = stmt->driver_data;
	pdo_tidb_db_handle *H = S->H;

	pdo_tidb_clear(H);
	if (!tidb_stmt_release_result(S)) {
		pdo_tidb_raise(stmt->dbh, stmt);
		return 0;
	}

	if (S->stmt) {
		S->result = tidb_stmt_run_native(stmt, S);
	} else {
		S->result = tidb_conn_query(H->conn, pdo_tidb_str(stmt->active_query_string), tidb_stmt_mode(S, false), 0, &H->err);
		if (!S->result) {
			pdo_tidb_raise(stmt->dbh, stmt);
		}
	}
	if (!S->result) {
		return 0;
	}
	tidb_stmt_adopt_result(stmt, S);
	return 1;
}

static int tidb_stmt_fetch(pdo_stmt_t *stmt, enum pdo_fetch_orientation ori, zend_long offset)
{
	pdo_tidb_stmt *S = stmt->driver_data;
	tidb_result_info info;
	int32_t ret;

	if (!S->result || stmt->column_count == 0) {
		return 0;
	}
	pdo_tidb_clear(S->H);
	ret = tidb_result_next(S->H->conn, S->result, &S->H->err);
	if (ret < 0) {
		pdo_tidb_raise(stmt->dbh, stmt);
		return 0;
	}
	tidb_result_get_info(S->result, &info);
	if (!info.has_row_count) {
		stmt->row_count = (zend_long) info.fetched;
	}
	return ret;
}

static int tidb_stmt_describe(pdo_stmt_t *stmt, int colno)
{
	pdo_tidb_stmt *S = stmt->driver_data;
	struct pdo_column_data *col = &stmt->columns[colno];
	tidb_column c;

	if (!S->result || !tidb_result_column(S->result, (size_t) colno, &c)) {
		return 0;
	}
	if (S->H->fetch_table_names && c.table.len) {
		col->name = zend_strpprintf(0, "%.*s.%.*s", (int) c.table.len, c.table.ptr, (int) c.name.len, c.name.ptr);
	} else {
		col->name = zend_string_init(c.name.ptr, c.name.len, 0);
	}
	col->maxlen = c.length;
	col->precision = c.decimals;
	return 1;
}

static int tidb_stmt_get_col(pdo_stmt_t *stmt, int colno, zval *result, enum pdo_param_type *type)
{
	pdo_tidb_stmt *S = stmt->driver_data;
	tidb_cell c;

	if (!S->result || !tidb_result_cell(S->result, (size_t) colno, &c)) {
		ZVAL_NULL(result);
		return 1;
	}
	pdo_tidb_cell_zval(&c, result);
	return 1;
}

void pdo_tidb_column_meta(const tidb_column *c, zval *out)
{
	static const struct {
		uint16_t flag;
		const char *name;
	} flag_names[] = {
		{TIDB_FLAG_NOT_NULL, "not_null"},
		{TIDB_FLAG_PRI_KEY, "primary_key"},
		{TIDB_FLAG_UNIQUE_KEY, "unique_key"},
		{TIDB_FLAG_MULTIPLE_KEY, "multiple_key"},
		{TIDB_FLAG_BLOB, "blob"},
		{TIDB_FLAG_UNSIGNED, "unsigned"},
		{TIDB_FLAG_ZEROFILL, "zerofill"},
		{TIDB_FLAG_BINARY, "binary"},
		{TIDB_FLAG_ENUM, "enum"},
		{TIDB_FLAG_AUTO_INCREMENT, "auto_increment"},
		{TIDB_FLAG_TIMESTAMP, "timestamp"},
		{TIDB_FLAG_SET, "set"},
		{TIDB_FLAG_ON_UPDATE_NOW, "on_update_now"},
	};
	bool integer = c->type_id == 1 || c->type_id == 2 || c->type_id == 3 || c->type_id == 8 || c->type_id == 9 || c->type_id == 13;
	/* TiDB does not set BLOB_FLAG on column definitions, so the TINY/MEDIUM/LONG/BLOB type ids decide. */
	bool blob = c->type_id >= 249 && c->type_id <= 252;
	zval flags;

	array_init(out);
	add_assoc_stringl(out, "native_type", c->native_type.ptr, c->native_type.len);
	add_assoc_long(out, "pdo_type", integer ? PDO_PARAM_INT : (blob && c->charset == TIDB_BINARY_CHARSET ? PDO_PARAM_LOB : PDO_PARAM_STR));
	array_init(&flags);
	for (size_t i = 0; i < sizeof(flag_names) / sizeof(flag_names[0]); i++) {
		if (c->flags & flag_names[i].flag) {
			add_next_index_string(&flags, flag_names[i].name);
		}
	}
	add_assoc_zval(out, "flags", &flags);
	add_assoc_stringl(out, "table", c->table.ptr, c->table.len);
	add_assoc_stringl(out, "name", c->name.ptr, c->name.len);
	add_assoc_long(out, "len", (zend_long) c->length);
	add_assoc_long(out, "precision", (zend_long) c->decimals);
	add_assoc_stringl(out, "tidb:org_name", c->org_name.ptr, c->org_name.len);
	add_assoc_stringl(out, "tidb:org_table", c->org_table.ptr, c->org_table.len);
	add_assoc_stringl(out, "tidb:schema", c->schema.ptr, c->schema.len);
	add_assoc_long(out, "tidb:charset", (zend_long) c->charset);
}

static int tidb_stmt_get_column_meta(pdo_stmt_t *stmt, zend_long colno, zval *return_value)
{
	pdo_tidb_stmt *S = stmt->driver_data;
	tidb_column c;

	if (!S->result || colno < 0 || !tidb_result_column(S->result, (size_t) colno, &c)) {
		return FAILURE;
	}
	pdo_tidb_column_meta(&c, return_value);
	return SUCCESS;
}

static int tidb_stmt_next_rowset(pdo_stmt_t *stmt)
{
	pdo_tidb_stmt *S = stmt->driver_data;
	int32_t ret;

	if (!S->result) {
		return 0;
	}
	pdo_tidb_clear(S->H);
	ret = tidb_result_next_rowset(S->H->conn, S->result, &S->H->err);
	if (ret < 0) {
		pdo_tidb_raise(stmt->dbh, stmt);
		return 0;
	}
	if (ret == 0) {
		return 0;
	}
	tidb_stmt_adopt_result(stmt, S);
	return 1;
}

static int tidb_stmt_cursor_closer(pdo_stmt_t *stmt)
{
	pdo_tidb_stmt *S = stmt->driver_data;

	pdo_tidb_clear(S->H);
	if (!tidb_stmt_release_result(S)) {
		pdo_tidb_raise(stmt->dbh, stmt);
		return 0;
	}
	return 1;
}

static int tidb_stmt_set_attr(pdo_stmt_t *stmt, zend_long attr, zval *val)
{
	pdo_tidb_stmt *S = stmt->driver_data;
	zend_long lval;
	bool bval;

	switch (attr) {
		case PDO_TIDB_ATTR_BUFFERED_QUERY:
			if (!pdo_get_bool_param(&bval, val)) {
				return 0;
			}
			S->buffered = bval;
			return 1;
		case PDO_TIDB_ATTR_CURSOR_FETCH_SIZE:
			if (!pdo_get_long_param(&lval, val) || lval < 0 || lval > UINT32_MAX) {
				return 0;
			}
			S->fetch_size = (uint32_t) lval;
			return 1;
		default:
			return 0;
	}
}

static int tidb_stmt_get_attr(pdo_stmt_t *stmt, zend_long attr, zval *val)
{
	pdo_tidb_stmt *S = stmt->driver_data;

	switch (attr) {
		case PDO_TIDB_ATTR_BUFFERED_QUERY:
			ZVAL_BOOL(val, S->buffered);
			return 1;
		case PDO_TIDB_ATTR_CURSOR_FETCH_SIZE:
			ZVAL_LONG(val, (zend_long) S->fetch_size);
			return 1;
		default:
			return 0;
	}
}

const struct pdo_stmt_methods tidb_stmt_methods = {
	tidb_stmt_dtor,
	tidb_stmt_execute_cb,
	tidb_stmt_fetch,
	tidb_stmt_describe,
	tidb_stmt_get_col,
	NULL, /* param_hook */
	tidb_stmt_set_attr,
	tidb_stmt_get_attr,
	tidb_stmt_get_column_meta,
	tidb_stmt_next_rowset,
	tidb_stmt_cursor_closer
};
