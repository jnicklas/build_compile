# BuildCompile

This is a small crate which allows you to hook compilers which compile to Rust
into your Cargo build scripts.

Suppose you have written some kind of DSL which compiles to rust code. Now you
want to provide users of your library a nice way to set up a Cargo build
script. With BuildCompile this is as simple as:

``` rust
extern crate build_compile as build;

use std::io::Write;
use std::path::Path;

struct Processor;

impl build::Processor for Processor {
    fn process<O: Write>(&self, input: build::FileText, output: &mut O) -> Result<(), build::Error> {
        try!(write!(output, "hello"));
        Err(build::Error(input, "it's not even implemented yet", build::Span(1, 3)));
    }
}

pub fn process_root() {
    build::process_root("someextension", &Processor)
}

pub fn process_dir<P: AsRef<Path>>(path: P) {
    build::process_dir(path, "someextension", &Processor)
}
```

Inside the `process` function you can do whatever you want. BuildCompile
handles all the directory traversing for you. It also generates nice looking
error message with highlighted errors for you.

Now users of your library can use your processor in their own crates with ease:

``` rust
extern crate some_crate;

fn main() {
    some_crate::process_root();
}
```

BuildCompile is extracted from
[LALRPOP](https://github.com/nikomatsakis/lalrpop) by Niko Matsakis.
