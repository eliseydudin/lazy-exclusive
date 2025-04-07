# lazy-exclusive
a global container type (like `LazyLock`) with runtime-checked mutability. can be used for `static` variables
```rust
let lazy = LazyExclusive::new(20);
let mut lock = lazy.get().unwrap(); // Mut<'_, i32>
let mut mutref = &mut *lock; // &mut i32
println!("{}", *mutref); // will print 20

let opt = lazy.get(); // is none because lock still exists
```
add this crate to your code like this:
```sh
cargo add lazy-exclusive
```

# use-locks
enable the `use-locks` feature for the crate to use system-implemented locks.

```rust
let start = Instant::now();
let five_seconds = Duration::from_secs(5);
static SHARED: LazyExclusive<i32> = LazyExclusive::new(120);
let mut lock = SHARED.get().unwrap();

std::thread::spawn(move || {
    *lock *= 2;
    std::thread::sleep(Duration::new(5, 0));
});

assert_eq!(SHARED.get_state(), State::Locked);
let new_lock = SHARED.wait();
assert_eq!(*new_lock, 120 * 2);
assert!(start.elapsed() >= five_seconds);
```

# no_std support
this crate supports no_std, which can be used by disabling default-features
```toml
[dependencies]
lazy-exclusive = { version = "1.0", default-features = false }
```
you can enable `use-locks` with `no_std` like this
```toml
[dependencies]
lazy-exclusive = { version = "1.0", default-features = false, features = ["use-locks"] }
```
note that this is not guaranteed to work properly
