# Instructions

First install Rust and Cargo, e.g. like this:

```
curl https://sh.rustup.rs -sSf | sh
```

Then run the code to search through different game playing policies:

```
cargo run --release
```

If the output looks like this, it's working:

```
204866 policies >= 30%, 10444 policies >= 50%
policy with combined-score [...]
```
