# timer.rs
rust timer crate.

only for linux with libc.

# Example

```rust
let (mut timer, rx) = Timer::new();
timer.ticker(CLOCK_REALTIME, 10).unwrap();
assert!(timer.get_id() != 0);
timer.start_reltime(Duration::from_millis(640), Duration::from_secs(3)).unwrap();
for _ in 0..5 {
    let overrun = rx.recv().unwrap();
    println!("overrun:{}", overrun);
}
```

# Output

```text
overrun:0
overrun:0
overrun:0
overrun:0
overrun:0
```
