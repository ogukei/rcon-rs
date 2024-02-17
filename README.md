# rcon-rs
A simple RCON client written in Rust

```
RCON_ENDPOINT="127.0.0.1:27015" RCON_PASSWORD="passwrd" RCON_COMMAND="/some command" RUST_LOG=trace cargo run --release
```

* RCON spec
  * https://developer.valvesoftware.com/wiki/Source_RCON_Protocol

## Implementation example
* Palworld player list observer
    *  https://github.com/ogukei/rcon-rs/tree/feature/palworld
