# Logging

NeoPRISM uses structured logging to help you diagnose issues and monitor node activity.
Logging is powered by the [`tracing`](https://docs.rs/tracing/latest/tracing/) crate, and log verbosity is controlled via the standard `RUST_LOG` environment variable.
By default, NeoPRISM outputs all logs to stdout.

## Configuring Logging

To set the log level, set the `RUST_LOG` environment variable before starting NeoPRISM. For example:

```bash
RUST_LOG=info
```

Supported log levels (in increasing verbosity) are: `error`, `warn`, `info`, `debug`, and `trace`.

You can also filter logs by module. For example, to see only HTTP-related logs at debug level:

```bash
RUST_LOG=neoprism_node::http=debug
```

Multiple filters can be combined:

```bash
RUST_LOG=info,oura=warn,neoprism_node::http=trace,tower_http::trace=debug
```

## About `RUST_LOG`

NeoPRISM uses the standard [`tracing`](https://docs.rs/tracing/latest/tracing/) environment variables to control log verbosity and filtering, including `RUST_LOG`. For more details on how `RUST_LOG` works and advanced usage, see the [tracing-subscriber EnvFilter documentation](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).
