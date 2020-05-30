bc4py-hash
====
bc4py crypto hash functions
* Yescrypt (Yespower0.5)
* Yespower0.9
* x16s
* x11
* poc (proof of capacity)

requirement
----
* MSVC on windows
* GCC on linux

usage
----
There are three features.

| name | note |
| ---- | ---- |
| hashs | enables yespower, x16s and x11 hash |
| poc  | enables poc hash functions |
| progress-bar | print poc work progress |

specify by features like this.
```
[dependencies]
bc4py_hash = {version = "0.1", features= ["hashs", "poc"]}
```

test
----
check all test except heavy test
```commandline
cargo test --features hashs --features poc
```

heavy test of *plot_file* by
```commandline
cargo test --features poc --release -- --ignored
```

sources
----
* [yespower](https://github.com/namuyan/yespower-python)
* [x16s](https://pypi.org/project/shield-x16s-hash/)
* [x11](https://pypi.org/project/x11_hash)

Author
----
[@namuyan_mine](https://twitter.com/namuyan_mine)

licence
----
MIT
