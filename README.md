# BindRS :file_folder::link::file_folder:

> Two way file syncer using platform native notify in Rust

Rebuilding https://github.com/daveallie/entangler in Rust.

## Installation

### Installation through cargo

1. Install [Rustup](https://rustup.rs/)
2. Run
```
cargo install --git https://github.com/daveallie/bindrs --tag v0.0.2
```

### Installing binary manually

1. Download the zipped binary for your platform from the [latest release](https://github.com/daveallie/bindrs/releases/latest) page
2. Copy or symlink the binary to `/usr/local/bin` or place it on your `PATH`.

## Usage

```
$ bindrs -h
BindRS 0.0.2
Two way file syncer using platform native notify in Rust

USAGE:
    bindrs <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    master    Runs BindRS in master mode - launches a slave
    slave     Runs BindRS in slave mode - launched from a master
```

---

```
$ bindrs master -h
bindrs-master 0.0.2
Runs BindRS in master mode - launches a slave

USAGE:
    bindrs master [FLAGS] [OPTIONS] <BASE DIR> <REMOTE DIR>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Log debug lines

OPTIONS:
    -i, --ignore <FILE>...    Ignores a file or regex match
    -p, --port <PORT>         Override SSH port (defaults to 22)

ARGS:
    <BASE DIR>      Local folder path
    <REMOTE DIR>    Local folder path or folder path over ssh (<remote_user>@<remote_host>:<remote_dir>)
```

TODO: Rest of usage docs

### Ignoring files

TODO: Ignoring files docs

## Contributing

1. Fork it!
- Create your feature branch: `git checkout -b my-new-feature`
- Commit your changes: `git commit -am 'Add some feature'`
- Push to the branch: `git push origin my-new-feature`
- Submit a pull request :D

### Development

1. Install [Vagrant](https://www.vagrantup.com/downloads.html)
- Navigate to the development directory
- Run `vagrant up`
- Run `vagrant ssh`
- Project will be in the `~/bindrs` folder
- Run `cargo build` to build the source

## License

The project is available as open source under the terms of the [MIT License](http://opensource.org/licenses/MIT).
