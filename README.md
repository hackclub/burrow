# Burrow

![License](https://img.shields.io/github/license/hackclub/burrow) ![Apple Build Status](https://img.shields.io/github/actions/workflow/status/hackclub/burrow/build-apple.yml?branch=main&label=macos%2C%20ios&logo=Apple) ![Crate Build Status](https://img.shields.io/github/actions/workflow/status/hackclub/burrow/build-rust.yml?branch=main&label=crate&logo=Rust)

Burrow is an open source tool for burrowing through firewalls, built by teenagers at [Hack Club](https://hackclub.com/).

`burrow` provides a simple command-line tool to open virtual interfaces and direct traffic through them.
Routine verification now runs unprivileged with `cargo test --workspace --all-features`; only tunnel startup needs elevation.

The repository now carries its own design and deployment record:

- [Constitution](./CONSTITUTION.md)
- [Agent Instructions](./AGENTS.md)
- [Burrow Evolution](./evolution/README.md)
- [WireGuard Rust Lineage](./docs/WIREGUARD_LINEAGE.md)
- [Protocol Roadmap](./docs/PROTOCOL_ROADMAP.md)
- [Forward Email Runbook](./docs/FORWARDEMAIL.md)

## Contributing

Burrow is fully open source, you can fork the repo and start contributing easily. For more information and in-depth discussions, visit the `#burrow` channel on the [Hack Club Slack](https://hackclub.com/slack/), here you can ask for help and talk with other people interested in burrow. Checkout [GETTING_STARTED.md](./docs/GETTING_STARTED.md) for build instructions and [GTK_APP.md](./docs/GTK_APP.md) for the Linux app. Forge and deployment scaffolding live in [`flake.nix`](./flake.nix), [`nixos/`](./nixos), and [`.forgejo/workflows/`](./.forgejo/workflows/). Hosted mail backup operations live in [`docs/FORWARDEMAIL.md`](./docs/FORWARDEMAIL.md) and [`Tools/forwardemail-custom-s3.sh`](./Tools/forwardemail-custom-s3.sh).

Agent and governance-sensitive work should start with [AGENTS.md](./AGENTS.md), [CONSTITUTION.md](./CONSTITUTION.md), and the relevant BEPs under [`evolution/proposals/`](./evolution/proposals/). Identity and bootstrap metadata now live in [`contributors.nix`](./contributors.nix).

The project structure is divided in the following folders: 

```
Apple/ # Xcode project for burrow on macOS and iOS
burrow/ # Higher-level API library for tun and tun-async
burrow-gtk/ # GTK project for burrow on Linux
tun/ # Low-level interface to OS networking
    src/
        tokio/ # Async/Tokio code
        unix/ # macOS and Linux code
        windows/ # Windows networking code
```

## Installation 

To start burrowing, download the latest release build in the release section. 

## Hack Club

Hack Club is a global community of high-school hackers from all around the world! Start your own Hack Club, attend an upcoming hackathon or join our online community at [hackclub.com](https://hackclub.com/).

## License 

Burrow is open source and licensed under the [GNU General Public License v3.0](./LICENSE.md)

## Contributors

<a href="https://github.com/hackclub/burrow/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=hackclub/burrow" />
</a>
