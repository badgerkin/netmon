# netmon

An htop-like live network usage monitor for the terminal, with a throughput chart, totals, and a live connection table.

## Install

Tarball:

```sh
sha256sum -c netmon-0.1.0-x86_64-linux-musl.tar.gz.sha256
tar xzf netmon-0.1.0-x86_64-linux-musl.tar.gz
install -m755 netmon-0.1.0-x86_64-linux-musl/netmon ~/.local/bin/
```

Or install to `/usr/local/bin` with `sudo`.

Debian/Ubuntu:

```sh
sudo apt install ./netmon_0.1.0-1_amd64.deb
```

## Requirements

- Linux x86_64. The binary is static, so there's no glibc version requirement.
- `ss` from iproute2. Without it the connections table stays empty. The .deb pulls it in.
- Run with `sudo` to see process names for other users' sockets.
- A truecolor terminal is recommended.
- Terminal size of at least 60x12.

## Keys

| Key | Action |
| --- | --- |
| `q` | quit |
| `s` / `S` | sort by next / previous column (any column; starts on RX) |
| `r` | reverse sort direction |
| `j`/`k`, arrows, `PgUp`/`PgDn`, `Home`/`End` | scroll connections |
| `f` | cycle filter: all / tcp / udp |
| `p` | toggle process (pid/prog) column |
| `d` | cycle refresh delay: 100 / 500 / 1000 ms |
| `h` | open help |
| `h` / `Esc` | close help |
| `Esc` | clear connection selection (stats lines cover all connections) |

## Building

```sh
rustup target add x86_64-unknown-linux-musl
cargo install cargo-deb --locked
scripts/dist.sh
```

`scripts/dist.sh` writes the tarball, its `.sha256`, and the `.deb` into `dist/`.

Plain `cargo build --release` gives a normal dynamically linked dev build.

## License

MIT, see [LICENSE](LICENSE).
