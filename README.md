# pollable-map

`pollable-map` provides keyed collections of futures and streams that can be
polled as a single stream. It also includes set variants, reusable optional
tasks, ordered futures, and optional timeout wrappers.

## Overview

- `FutureMap` polls a collection of futures assigned to a specific key.
- `StreamMap` polls a collection of streams assigned to a specific key.
- `FutureSet` polls futures without assigning user-provided keys.
- `StreamSet` combines streams without exposing internal keys.
- `OrderedFutureSet` polls futures one at a time in FIFO order.
- `Optional<T>` is similar to `Option<T>` but is a reusable future or stream
  that can be empty and later receive another task.
- Timeout map, set, and optional wrappers are available with the `timeout` feature.

Maps and optional tasks remain reusable after becoming empty. Set
`FutureMap::set_terminate_on_empty` or `StreamMap::set_terminate_on_empty` to true to make it behave as a
terminated stream after all current entries complete.

## Examples

### FutureMap example

```rust
use futures::future::BoxFuture;
use futures::StreamExt;
use pollable_map::futures::FutureMap;

fn main() {
    futures::executor::block_on(async {
        let mut fut_map = FutureMap::<u8, BoxFuture<'static, ()>>::new();

        fut_map.insert(
            1,
            Box::pin(async {
                println!("message from 1");
            }),
        );
        fut_map.insert(
            2,
            Box::pin(async {
                println!("message from 2");
            }),
        );

        while let Some((key, _)) = fut_map.next().await {
            println!("> future \"{}\" is finished", key);
        }

        println!("Done processing map")
    });
}

```

### StreamMap example

```rust
use futures::stream::BoxStream;
use futures::StreamExt;
use pollable_map::stream::StreamMap;

fn main() {
    futures::executor::block_on(async {
        let mut st_map = StreamMap::<u8, BoxStream<'static, ()>>::new();

        st_map.insert(
            1,
            futures::stream::once(async {
                println!("message from 1");
            })
                .boxed(),
        );
        st_map.insert(
            2,
            futures::stream::once(async {
                println!("message from 2");
            })
                .boxed(),
        );

        while let Some((key, _)) = st_map.next().await {
            println!("> stream \"{}\" is finished", key);
        }

        println!("Done processing map")
    });
}
```

## License

Dual licensed under MIT or Apache License (Version 2.0). See [LICENSE-MIT](./LICENSE-MIT) and
[LICENSE-APACHE](./LICENSE-APACHE) for more details. You may choose the license that best fits your use case.
