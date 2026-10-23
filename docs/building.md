# Building and installing pdo_tidb

The extension has two parts:

- `ffi/`: a Rust static library (`libtidb_ffi.a`) with the protocol, authentication, TLS and compression, behind a C ABI.
- `src/`: the C code of the PDO driver, built with the usual `phpize` / `configure` / `make` flow.

`make` runs `cargo build --release --locked` in `ffi/` before it links the extension, and relinks the extension whenever cargo rebuilt the library, so the normal build needs nothing beyond the prerequisites below.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| PHP | 8.4 or newer, with `pdo` | Headers and `phpize` come from `php-dev` (Debian, Ubuntu), `php-devel` (Fedora, RHEL) or Homebrew `php`. |
| Rust | 1.88 or newer | Install with [rustup](https://rustup.rs). `cargo` must be on `PATH` when `configure` runs. |
| C toolchain | any recent `cc`, `make`, `autoconf` | Xcode Command Line Tools on macOS, `build-essential autoconf` on Debian. |
| Network | first build only | Cargo downloads the crates listed in `ffi/Cargo.lock`. |

Debian or Ubuntu:

```sh
sudo apt-get install -y php-dev build-essential autoconf curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
```

macOS with Homebrew:

```sh
xcode-select --install
brew install php autoconf rustup
rustup-init -y
```

## Option 1: PECL

Install straight from a release tarball:

```sh
pecl install https://github.com/stringke/tidb-php/releases/download/v0.1.1/pdo_tidb-0.1.1.tgz
```

or from a tarball you built yourself (see [development.md](development.md#packaging)):

```sh
pecl install ./pdo_tidb-0.1.1.tgz
```

PECL asks for the `with-tidb-ffi` directory; press Enter to keep `no` and build the Rust part with cargo. When PECL runs through `sudo`, the root environment usually has no `cargo`; either keep Rust on root's `PATH` (`sudo env "PATH=$PATH" pecl install ...`) or build as your user with option 3 and install with `sudo make install`.

## Option 2: PIE

[PIE](https://github.com/php/pie) builds from the tagged sources in this repository:

```sh
pie install stringke/tidb-php
```

If the package is not on Packagist yet, register the repository first:

```sh
pie repository:add vcs https://github.com/stringke/tidb-php
pie install stringke/tidb-php:^0.1
```

PIE enables the extension in `php.ini` for you. A prebuilt Rust library is passed as `pie install stringke/tidb-php --with-tidb-ffi=/opt/tidb-ffi`.

## Option 3: from source

```sh
git clone https://github.com/stringke/tidb-php.git
cd tidb-php
phpize
./configure
make
make test            # optional, needs a running TiDB, see development.md
sudo make install
```

Everything the build generates is ignored by git; `phpize --clean` removes it.

When several PHP versions are installed, pick one with `phpize8.4` / `--with-php-config=/usr/bin/php-config8.4`, or the Homebrew paths under `$(brew --prefix php@8.4)/bin`.

## Enabling the extension

`make install` copies `pdo_tidb.so` into `php-config --extension-dir`. Load it after `pdo`:

```ini
; for example /etc/php/8.4/mods-available/pdo_tidb.ini, or conf.d/ext-pdo_tidb.ini
extension=pdo_tidb
```

Check:

```sh
php -m | grep pdo_tidb
php --ri pdo_tidb
php -r 'var_dump(in_array("tidb", PDO::getAvailableDrivers()));'
```

`pdo_tidb` and `pdo_mysql` can be loaded side by side; they register different DSN prefixes (`tidb:` and `mysql:`).

## Configure options

| Option | Default | Meaning |
|---|---|---|
| `--enable-pdo-tidb` | enabled | Build the driver. `--disable-pdo-tidb` turns it off. |
| `--with-tidb-ffi=DIR` | not set | Link `DIR/libtidb_ffi.a` instead of building it with cargo. |

### Prebuilt Rust library

For hosts without Rust or without network access, build the library once on a machine with the same OS, architecture and C library, then link it:

```sh
# build machine
cd tidb-php/ffi
cargo build --release --locked
cp target/release/libtidb_ffi.a /opt/tidb-ffi/

# target machine
cd tidb-php
phpize
./configure --with-tidb-ffi=/opt/tidb-ffi
make && sudo make install
```

The library only depends on the system libraries listed in `config.m4` (`pthread`, `dl`, `m`, `rt` on Linux; `CoreFoundation`, `Security`, `SystemConfiguration`, `iconv`, `resolv` on macOS). TLS uses rustls, so OpenSSL is not needed; certificate verification uses the operating system's trust store unless `ssl_ca` or `ssl_capath` is given.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `configure: error: cargo is required ...` | `cargo` is not on `PATH` for the shell that runs `configure` (often `sudo`). Install Rust or pass `--with-tidb-ffi`. |
| `rustc 1.xx is not supported by ...` | Rust is older than 1.88. Run `rustup update`. |
| Cargo fails to download crates | The first build needs crates.io. Configure a mirror in `~/.cargo/config.toml`, or use a prebuilt library. |
| The extension still behaves like an older build after a change in `ffi/` | The `Makefile` was generated before `Makefile.frag` changed. Run `./configure` again (not `./config.status`, which reuses the old fragment). |
| `phpize: Permission denied` on files such as `build/Makefile.global` | A previous `phpize` copied read-only files. Run `phpize --clean` and start again. |
| `PDO::__construct(): could not find driver` | The extension is not loaded, or it is loaded before `pdo`. Check `php --ini` and the order of the `extension=` lines. |

## Uninstall

```sh
pecl uninstall pdo_tidb       # when installed with PECL
rm "$(php-config --extension-dir)/pdo_tidb.so"   # otherwise
```

and remove the `extension=pdo_tidb` line.
