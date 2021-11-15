#[no_mangle]
fn caller_fn(a: u32, b: u32) -> u32 {
    inlined_fn(a, b)
}

#[inline(always)]
fn inlined_fn(a: u32, b: u32) -> u32 {
    let a2 = a + a + 2;
    let b2 = b + b + 2;
    let res = a2 + b2;

    res
}
