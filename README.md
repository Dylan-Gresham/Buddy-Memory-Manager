# Buddy Memory Manager

Steps to configure, build, and test the project.

## System Build Requirements

To build the C bindings, `cbindgen` is required and can be installed with the below command.

```bash
cargo install --force cbindgen
```

The `--force` option makes it update to the latest version of `cbindgen` if it is already present on the system.

## Building

```bash
make build
```

## Testing

```bash
make check
```

## Clean

```bash
make clean
```

## Generate Documentation

Using the below command will generate the documentation and open it in your system's default browser.

```bash
make docs
```

Using the below command will generate the documentation under `target/doc` and will **_not_** open it automatically.

```bash
make docs-no-open
```

## Install Dependencies

In order to use git send-mail you need to run the following command:

```bash
make install-deps
```
