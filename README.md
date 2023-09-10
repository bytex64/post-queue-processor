# `post-queue-processor`

This is a simple tool which listens for uploads on its root, stores the
files, and runs a command on the uploaded files. It debounces the
incoming uploads so that the command is not executed after every upload.

## Prerequisites

It uses Rocket, Serde, and Toml, which should be fetched by Cargo. No
external dependencies are required.

## Configuration

All HTTP-side configuration is done via Rocket.toml. Please see the
[Rocket configuration
docs](https://rocket.rs/v0.5-rc/guide/configuration/) for more
information.

Configuring `post-queue-processor` itself is done via a `config.toml` in
the working directory of the program. See
[config.toml.example](config.toml.example) for
information on how it works.

## Usage

Once a `config.toml` is configured, simply run (`cargo run`) to launch
the server.
