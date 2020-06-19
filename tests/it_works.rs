use pahole_macro::pahole;

#[pahole]
struct A {
    b: usize,
    c: i8,
}
