# lazy-exclusive
a global container type (like `LazyLock`) with runtime-checked mutability
```rust
let lazy = LazyExclusive::new(20);
let mut lock = lazy.get().unwrap(); // Mut<'_, i32>
let mut mutref = &mut *lock; // &mut i32
println!("{}", *mutref); // will print 20

let opt = lazy.get(); // is none because lock still exists
```
add this crate to your code like this:
```toml
[dependencies]
lazy-exclusive = { git = "https://github.com/eliseydudin/lazy-exclusive.git" }
```

# use-locks
you can wait for `LazyExclusive` to unlock by enabling the `use-locks` feature and using the `wait` method.

```rust
static SHARED: LazyExclusive<i32> = LazyExclusive::new(120);
let mut lock = SHARED.get().unwrap();

std::thread::spawn(move || {
    *lock *= 2;
    std::thread::sleep(Duration::new(5, 0));
});

assert_eq!(SHARED.get_state(), State::Locked);
let new_lock = SHARED.wait();
assert_eq!(*new_lock, 120 * 2);
```
