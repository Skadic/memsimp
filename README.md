# memsimp

Track the approximate memory usage of any program at runtime using `proc statm`.

## Usage

Build using `cargo build` or `cargo build --release`. The binary is then in `target/<debug|release>`.
Run `memsimpl --help` to get a basic overview of how to use it.

```sh
Usage: memsimp [OPTIONS] [APP]...

Arguments:
  [APP]...  The program to run including arguments

Options:
  -s, --sample-rate <SAMPLE_RATE>  The number of milliseconds to wait between each sample
  -t, --timeout <TIMEOUT>          The number of milliseconds to wait before starting to collect samples
  -h, --help                       Print help
  -V, --version                    Print version
```

For example:
``` sh
./memsimp sleep 10
```
The output is something like:
```
Peak heap kilo bytes: 2128
```

This will use a default sample rate of one sample per 100ms and timeout of 0ms.

If you wish to use a custom sample rate and/or timeout, you can use:
``` sh
./memsimp -t 1000 -s 200 -- sleep 10
```
This will wait 1000ms before starting to collect samples and them use a sample rate of 200ms.

