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
