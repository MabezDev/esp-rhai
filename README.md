# esp-rhai

Running [`rhai`](https://github.com/rhaiscript/rhai) on an esp. This example is specifically targeted for the esp32c3 based [esp-rust-board](https://github.com/esp-rs/esp-rust-board), but will work on any esp32 with a bit of massaging.

## Currently implemented commands

| Command | Usage                                                      |
|---------|------------------------------------------------------------|
| print   | print("Hello world!") or print(`Value = ${some_variable}`) |
| heap    | heap()                                                     |