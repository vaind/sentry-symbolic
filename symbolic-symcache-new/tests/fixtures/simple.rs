#[no_mangle]
fn simple_fn(a: u32, b: u32) -> u32 {
    let a2 = a + a + 2;
    let b2 = b + b + 2;
    let res = a2 + b2;

    res
}
