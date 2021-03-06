// code from the ruffle-rs project
/*
Copyright (c) 2018 Mike Welsh <mwelsh@gmail.com>

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
*/
/// Converts an `f64` to an `u32` with ECMAScript `ToUInt32` wrapping behavior.
/// The value will be wrapped modulo 2^32.
#[allow(clippy::unreadable_literal)]
pub fn f64_to_wrapping_u32(n: f64) -> u32 {
    if !n.is_finite() {
        0
    } else {
        n.trunc().rem_euclid(4294967296.0) as u32
    }
}
/// Converts an `f64` to an `i32` with ECMAScript `ToInt32` wrapping behavior.
/// The value will be wrapped in the range [-2^31, 2^31).
pub fn f64_to_wrapping_i32(n: f64) -> i32 {
    f64_to_wrapping_u32(n) as i32
}
