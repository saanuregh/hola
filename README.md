<!--
 Copyright (c) 2020 saanuregh

 This software is released under the MIT License.
 https://opensource.org/licenses/MIT
-->

# Hola

Windows Hello™ style facial authentication for Linux written in Rust, based on [Howdy](https://github.com/boltgolt/howdy). Uses [Linux Pluggable Authentication Modules (PAM)](https://wiki.archlinux.org/index.php/PAM) to provide a framework for system-wide user authentication. Uses `video4linux` for video capture.

## Installation

Packages for various distros is in WIP.

### Requires

- [dlib](http://dlib.net/)

To build and install with `cargo-make` just run `cargo make install_release`.

## Configuration

### Setup Hola to start when needed

In order for Hola to authenticate a user, a small change must be added to any PAM configuration file where Howdy might want to be used. The following line must be added to any configuration file:

`auth sufficient pam_hola.so`

To enable Hola authentication for `sudo`, add to `/etc/pam.d/sudo` file. Or to enable Hola authentication for graphical login add to `/etc/pam.d/system-local-login`.

### Configuration file

Configuration file is very similar in structure to Howdy's. To access it run `sudo hola config`, this command opens the configuration file in default editor. The configuration file is located at `/lib/security/pam_hola/config.toml`.

### Adding a face

To add a face to Hola, run `sudo hola model add`

## CLI commands

To see all the CLI command, run `sudo hola help`

## Caveats

This package is in no way as secure as a password and will never be, do not use this as your sole authentication method. To minimize the chance of this program being compromised like Howdy, it's recommended to leave Hola in /lib/security and to keep it read-only.
