.PHONY: tidb-ffi-always

tidb-ffi-always:

# cargo decides whether the static library is stale, so its recipe always runs; make then re-reads the mtime to decide on relinking.
$(TIDB_FFI_LIB): tidb-ffi-always
	$(TIDB_FFI_BUILD)
