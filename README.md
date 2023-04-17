# Burrow

![License](https://img.shields.io/github/license/hackclub/burrow) ![Apple Build Status](https://img.shields.io/github/actions/workflow/status/hackclub/burrow/build-apple.yml?branch=main&label=macos%2C%20ios&logo=Apple) ![Crate Build Status](https://img.shields.io/github/actions/workflow/status/hackclub/burrow/build-rust.yml?branch=main&label=crate&logo=Rust)

Burrow is an open source tool for burrowing through firewalls, built by teenagers at [Hack Club](https://hackclub.com/).

`burrow` provides a simple command-line tool to open virtual interfaces and direct traffic through them.

## Contributing

Burrow is fully open source, you can fork the repo and start contributing easily. For more information and in-depth discussions, visit the `#burrow` channel on the [Hack Club Slack](https://hackclub.com/slack/), here you can ask for help and talk with other people interested in burrow! For more information on how to contribute, please see [CONTRIBUTING.md]

The project structure is divided in the following folders: 

```
Apple/ # Xcode project for burrow on macOS and iOS
burrow/ # Higher-level API library for tun and tun-async
tun/ # Low-level interface to OS networking
    src/
        unix/ # macOS and Linux code
        windows/ # Windows networking code
tun-async/ # Async interface to tun
```

## Installation 

To start burrowing, download the latest release build in the release section. 

## Hack Club

Hack Club is a global community of high-school hackers from all around the world! Start your hack club by visiting the [Hack Club Page](https://hackclub.com/)

## License 

Burrow is open source and licensed under the [GNU General Public License v3.0](./LICENSE.md)

## Contributors

<a href="https://github.com/hackclub/burrow/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=hackclub/burrow" />
</a>
