mod bindings;

use bindings::Bindings;
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let library_path = env::args().skip(1).next().expect("USAGE: test <dylib>");

    let vtable = Bindings::load_from_path(&library_path)?;

    unsafe {
        let got = vtable.smoke_test_add(1, 2);
        assert_eq!(got, 3);
    }

    Ok(())
}
