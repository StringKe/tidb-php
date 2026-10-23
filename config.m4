PHP_ARG_ENABLE([pdo-tidb],
  [whether to enable PDO TiDB support],
  [AS_HELP_STRING([--enable-pdo-tidb],
    [Enable the PDO driver for TiDB])],
  [yes])

PHP_ARG_WITH([tidb-ffi],
  [for a prebuilt tidb_ffi static library],
  [AS_HELP_STRING([[--with-tidb-ffi[=DIR]]],
    [Directory containing a prebuilt libtidb_ffi.a; by default make builds it with cargo])],
  [no],
  [no])

if test "$PHP_PDO_TIDB" != "no"; then
  PHP_CHECK_PDO_INCLUDES

  dnl PHP's bundled libtool picks linker flags that current ld64 reports as deprecated
  dnl (-flat_namespace -undefined suppress when no deployment target is set, and -single_module).
  case $host_os in
    darwin*)
      if test -z "$MACOSX_DEPLOYMENT_TARGET"; then
        MACOSX_DEPLOYMENT_TARGET=`sw_vers -productVersion | cut -d. -f1-2`
        export MACOSX_DEPLOYMENT_TARGET
      fi
      lt_cv_apple_cc_single_mod=no
      TIDB_FFI_SYSTEM_LIBS="-framework CoreFoundation -framework Security -framework SystemConfiguration -liconv -lresolv"
      ;;
    *)
      dnl PHP dlclose()s extensions at shutdown, before exit() runs the Rust thread-local destructors;
      dnl -z nodelete keeps the module mapped so those destructors do not jump into unmapped code.
      TIDB_FFI_SYSTEM_LIBS="-lpthread -ldl -lm -lrt -Wl,-z,nodelete"
      ;;
  esac

  if test "$PHP_TIDB_FFI" = "no" || test "$PHP_TIDB_FFI" = "yes"; then
    AC_PATH_PROG([CARGO], [cargo], [no])
    if test "$CARGO" = "no"; then
      AC_MSG_ERROR([cargo is required to build the Rust part of pdo_tidb; install Rust 1.88 or newer from https://rustup.rs, or pass --with-tidb-ffi=DIR])
    fi
    TIDB_FFI_LIB="$abs_srcdir/ffi/target/release/libtidb_ffi.a"
    TIDB_FFI_BUILD="cd '$abs_srcdir/ffi' && '$CARGO' build --release --locked"
  else
    TIDB_FFI_LIB="$PHP_TIDB_FFI/libtidb_ffi.a"
    if test ! -f "$TIDB_FFI_LIB"; then
      AC_MSG_ERROR([$TIDB_FFI_LIB does not exist])
    fi
    TIDB_FFI_BUILD="true"
  fi

  dnl Passed through -Wl so libtool does not warn about linking a static archive into a shared module.
  PDO_TIDB_SHARED_LIBADD="-Wl,$TIDB_FFI_LIB $TIDB_FFI_SYSTEM_LIBS"
  PDO_TIDB_SHARED_DEPENDENCIES="$TIDB_FFI_LIB"
  PHP_SUBST([TIDB_FFI_LIB])
  PHP_SUBST([TIDB_FFI_BUILD])
  PHP_SUBST([PDO_TIDB_SHARED_LIBADD])
  PHP_SUBST([PDO_TIDB_SHARED_DEPENDENCIES])

  PHP_NEW_EXTENSION([pdo_tidb],
    [src/pdo_tidb.c src/tidb_driver.c src/tidb_statement.c],
    [$ext_shared],,
    [-I@ext_srcdir@/src])
  PHP_ADD_BUILD_DIR([$ext_builddir/src])
  PHP_ADD_EXTENSION_DEP(pdo_tidb, pdo)
  PHP_ADD_MAKEFILE_FRAGMENT
fi
