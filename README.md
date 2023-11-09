# memsimp

Track the approximate memory usage of any program at runtime using `proc statm`.

## Usage

Build using `cargo build` or `cargo build --release`. The binary is then in `target/<debug|release>`.

Then run using:
``` sh
./memsimp <my_binary> <myarg> 
```
For example:
``` sh
./memsimp sleep 10
```
The output is something like:
```
Peak heap kilo bytes: 2128
```

This will use a default sample rate of one sample per 100ms.

If you wish to use a custom sample rate, you can use:
``` sh
./memsimp 200 -- sleep 10
```
This will use a sample rate of 200ms.

