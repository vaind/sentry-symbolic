#[no_mangle]
fn caller_fn(a: u32, b: u32, c: u32, d: u32) -> u32 {
    let a = inlined_fn(a, b);
    let b = inlined_fn(c, d);
    a + b
}

#[inline(always)]
fn inlined_fn(a: u32, b: u32) -> u32 {
    let a = second_inline_fn(a);
    let b = second_inline_fn(b);
    a + b
}

#[inline(always)]
fn second_inline_fn(a: u32) -> u32 {
    if a > 10 {
        123
    } else {
        456
    }
}
