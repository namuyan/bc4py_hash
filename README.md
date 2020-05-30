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

test
----
```commandline
cargo test
```

heavy test *plot_file* by
```commandline
cargo test --release -- --ignored
```

install
----
```commandline
cargo install bc4py_hash
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
