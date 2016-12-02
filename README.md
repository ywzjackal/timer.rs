# timer.rs
rust timer crate.

only for linux with libc.

# Example

```rust
fn test_timer() {
    use std::thread;
    use std::time::Duration;
    use std::sync::*;
    let counter = Arc::new(Mutex::new(0));
    let counter1 = counter.clone();
    let mut timer = Timer::new();
    timer.on_arrived.join(move |overrun| {
        println!("counter:{}, overrun:{}", *counter1.lock().unwrap(), overrun);
        *counter1.lock().unwrap() += 1;
    });
    if let Ok(_) = timer.ticker(CLOCK_REALTIME, 50) {
        assert!(timer.get_id() != 0);
        if let Ok(_) = timer.start_reltime(Duration::from_millis(640), Duration::from_secs(1)) {
            thread::sleep(Duration::from_millis(640 * 4 + 100));
        }
    }
    assert!(*counter.lock().unwrap() >= 2);
}
```

# Output

```text
    Finished debug [unoptimized + debuginfo] target(s) in 0.0 secs
     Running target/debug/deps/timer-868c7c224cdbba4d
running 1 test
counter:0, overrun:0
counter:1, overrun:0
counter:2, overrun:0
test linux::test_timer ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
   Doc-tests timer
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured
```
