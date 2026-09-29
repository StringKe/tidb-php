/* C declarations of the Rust library in ffi/ (tidb_ffi). Keep in sync with ffi/src/abi.rs. */
#ifndef TIDB_FFI_H
#define TIDB_FFI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct tidb_conn tidb_conn;
typedef struct tidb_stmt tidb_stmt;
typedef struct tidb_result tidb_result;

/* Borrowed bytes; ptr == NULL means "absent". */
typedef struct {
	const char *ptr;
	size_t len;
} tidb_str;

/* Owned by the library, NUL terminated; release with tidb_owned_free(). */
typedef struct {
	char *ptr;
	size_t len;
} tidb_owned;

typedef struct {
	char sqlstate[6];
	int32_t code;
	int32_t error_class;
	bool fatal;
	tidb_owned message;
} tidb_error;

typedef struct {
	tidb_str key;
	tidb_str value;
} tidb_option;

enum {
	TIDB_MODE_BUFFERED = 0,
	TIDB_MODE_STREAMING = 1,
	TIDB_MODE_CURSOR = 2,
};

enum {
	TIDB_PARAM_NULL = 0,
	TIDB_PARAM_INT = 1,
	TIDB_PARAM_FLOAT = 2,
	TIDB_PARAM_BYTES = 3,
	TIDB_PARAM_LONG_DATA = 4,
};

typedef struct {
	int32_t kind;
	int64_t int_value;
	double float_value;
	tidb_str data;
} tidb_param;

enum {
	TIDB_CELL_NULL = 0,
	TIDB_CELL_INT = 1,
	TIDB_CELL_FLOAT = 2,
	TIDB_CELL_BYTES = 3,
};

/* data is valid until the next tidb_result_next() or tidb_result_free(). */
typedef struct {
	int32_t kind;
	int64_t int_value;
	double float_value;
	tidb_str data;
} tidb_cell;

/* Strings are valid until tidb_result_free(). */
typedef struct {
	tidb_str name;
	tidb_str org_name;
	tidb_str table;
	tidb_str org_table;
	tidb_str schema;
	tidb_str native_type;
	uint8_t type_id;
	uint8_t decimals;
	uint16_t flags;
	uint16_t charset;
	uint32_t length;
} tidb_column;

typedef struct {
	uint64_t affected_rows;
	uint64_t last_insert_id;
	uint16_t warnings;
	uint16_t status;
	bool more_results;
	bool is_cursor;
	bool has_row_count;
	uint64_t row_count;
	uint64_t fetched;
	tidb_str info;
} tidb_result_info;

enum {
	TIDB_ACTION_PING = 0,
	TIDB_ACTION_RESET_CONNECTION = 1,
	TIDB_ACTION_KILL_QUERY = 2,
	TIDB_ACTION_MULTI_STATEMENTS_ON = 3,
	TIDB_ACTION_MULTI_STATEMENTS_OFF = 4,
};

enum {
	TIDB_TEXT_SERVER_VERSION = 0,
	TIDB_TEXT_ENDPOINT = 1,
	TIDB_TEXT_AUTH_PLUGIN = 2,
	TIDB_TEXT_TLS_VERSION = 3,
	TIDB_TEXT_TLS_CIPHER = 4,
	TIDB_TEXT_COMPRESSION = 5,
	TIDB_TEXT_LAST_INFO = 6,
	TIDB_TEXT_DISCOVERY_ERROR = 7,
	TIDB_TEXT_DATABASE = 8,
};

enum {
	TIDB_NUM_CONNECTION_ID = 0,
	TIDB_NUM_STATUS = 1,
	TIDB_NUM_WARNINGS = 2,
	TIDB_NUM_AFFECTED_ROWS = 3,
	TIDB_NUM_LAST_INSERT_ID = 4,
	TIDB_NUM_CAPABILITIES = 5,
	TIDB_NUM_IN_TRANSACTION = 6,
	TIDB_NUM_HAS_PENDING_RESULT = 7,
	TIDB_NUM_BROKEN = 8,
};

/* Column definition flags. */
#define TIDB_FLAG_NOT_NULL 0x0001
#define TIDB_FLAG_PRI_KEY 0x0002
#define TIDB_FLAG_UNIQUE_KEY 0x0004
#define TIDB_FLAG_MULTIPLE_KEY 0x0008
#define TIDB_FLAG_BLOB 0x0010
#define TIDB_FLAG_UNSIGNED 0x0020
#define TIDB_FLAG_ZEROFILL 0x0040
#define TIDB_FLAG_BINARY 0x0080
#define TIDB_FLAG_ENUM 0x0100
#define TIDB_FLAG_AUTO_INCREMENT 0x0200
#define TIDB_FLAG_TIMESTAMP 0x0400
#define TIDB_FLAG_SET 0x0800
#define TIDB_FLAG_ON_UPDATE_NOW 0x2000

/* Server status flags. */
#define TIDB_STATUS_AUTOCOMMIT 0x0002

#define TIDB_BINARY_CHARSET 63

void tidb_owned_free(tidb_owned *owned);
void tidb_error_clear(tidb_error *error);
const char *tidb_driver_version(void);
const char *tidb_error_class_name(int32_t error_class);
int32_t tidb_error_class_of(int32_t code, bool connection_lost);

tidb_conn *tidb_connect(tidb_str dsn, tidb_str user, tidb_str password, const tidb_option *options, size_t count, tidb_error *error);
void tidb_conn_free(tidb_conn *conn);

tidb_result *tidb_conn_query(tidb_conn *conn, tidb_str sql, int32_t mode, uint32_t fetch_size, tidb_error *error);
bool tidb_conn_exec(tidb_conn *conn, tidb_str sql, uint64_t *affected, tidb_error *error);
tidb_stmt *tidb_conn_prepare(tidb_conn *conn, tidb_str sql, tidb_error *error);
bool tidb_conn_action(tidb_conn *conn, int32_t action, tidb_error *error);
bool tidb_conn_init_db(tidb_conn *conn, tidb_str database, tidb_error *error);
bool tidb_conn_change_user(tidb_conn *conn, tidb_str user, tidb_str password, tidb_str database, tidb_error *error);
bool tidb_conn_statistics(tidb_conn *conn, tidb_owned *out, tidb_error *error);
bool tidb_conn_refresh(tidb_conn *conn, uint8_t options, tidb_error *error);
tidb_result *tidb_conn_field_list(tidb_conn *conn, tidb_str table, tidb_str wildcard, tidb_error *error);
void tidb_conn_set_infile_data(tidb_conn *conn, tidb_str data);
bool tidb_conn_quote(const tidb_conn *conn, tidb_str value, bool binary, tidb_owned *out);
bool tidb_conn_text(const tidb_conn *conn, int32_t what, tidb_owned *out);
uint64_t tidb_conn_number(const tidb_conn *conn, int32_t what);
bool tidb_conn_alive(tidb_conn *conn);

size_t tidb_stmt_param_count(const tidb_stmt *stmt);
size_t tidb_stmt_column_count(const tidb_stmt *stmt);
tidb_result *tidb_stmt_execute(tidb_conn *conn, const tidb_stmt *stmt, const tidb_param *params, size_t count, int32_t mode, uint32_t fetch_size, tidb_error *error);
void tidb_stmt_free(tidb_conn *conn, tidb_stmt *stmt);

size_t tidb_result_column_count(const tidb_result *result);
bool tidb_result_column(const tidb_result *result, size_t index, tidb_column *out);
/* 1 row, 0 end of the current result set, -1 error. */
int32_t tidb_result_next(tidb_conn *conn, tidb_result *result, tidb_error *error);
bool tidb_result_cell(const tidb_result *result, size_t index, tidb_cell *out);
/* 1 next result set is current, 0 none left, -1 error. */
int32_t tidb_result_next_rowset(tidb_conn *conn, tidb_result *result, tidb_error *error);
bool tidb_result_close(tidb_conn *conn, tidb_result *result, tidb_error *error);
bool tidb_result_get_info(const tidb_result *result, tidb_result_info *out);
void tidb_result_free(tidb_result *result);

#endif
