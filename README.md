# thunky

[![crates.io version][1]][2] [![build status][3]][4] [![downloads][5]][6]

Delay the evaluation of a paramless async function and cache the result. Adapted from [mafintosh/thunky](https://github.com/mafintosh/thunky)

- [Documetaion](https://docs.rs/thunky)
- [Crates.io](https://crates.io/crates/thunky)

## Example

Let's make a simple function that returns a random number 1 second after it is called for the first time

```rust
extern crate thunky;
extern crate rand;
use thunky::*;
use rand::Rng;

fn main () {
    let run = move |thunk: &Thunky<u32, &str>| {
        let mut rng = rand::thread_rng();
        thunk.cache(Ok(rng.gen::<u32>()));
    };

    let thunk = Thunky::new(Box::new(run));

    // I miss JavaScript's `setTimeout()`
    let thunk_clone = Arc::clone(&thunk);
    let when = Instant::now() + Duration::from_millis(1000);
    let task = Delay::new(when)
        .and_then(move |_| {
           let mut rng = rand::thread_rng();
           thunk_clone.cache(Ok(rng.gen::<u32>()));
           Ok(())
        })
        .map_err(|e| panic!("delay errored; err={:?}", e));

    thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
        println!("{}", arg.unwrap()); // prints random number
    }));

    tokio::run(task);  

    thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
        println!("{}", arg.unwrap()); // prints the same random number as above
    }));
}
```

## Error → No caching

If the thunk cache function is called with an `Err<E>`, it will not cache the result

```rust 

let v = Mutex::new(0);

let run = move |thunk: &Thunky<u32, &str>| {
    if *v.lock().unwrap() == 0 {
        thunk.cache(Err("not cache"))
    } else if  *v.lock().unwrap() == 1 {
        thunk.cache(Ok(100))
    }
    *v.lock().unwrap() += 1;
}

let thunk = Thunky::new(Box::new(run));

thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
    assert_eq!("not cache", arg.unwrap_err());
}))

thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
    assert_eq!(100, arg.unwrap());
}))
```

## Installation

```sh
cargo add thunky
```

## License

[MIT](./LICENSE-MIT) OR [APACHE](./LICENSE-APACHE)

[1]: https://img.shields.io/crates/v/thunky.svg?style=flat-square
[2]: https://crates.io/crates/thunky
[3]: https://api.travis-ci.org/datrs/thunky.svg?branch=master
[4]: https://travis-ci.org/datrs/thunky
[5]: https://img.shields.io/crates/d/thunky.svg?style=flat-square
[6]: https://crates.io/crates/thunky