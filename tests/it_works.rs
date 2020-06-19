use pahole_macro::pahole;

#[pahole]
struct A {
    b: usize,
    c: i8,
}

#[pahole]
mod M {
    enum E {
        A,
        B(u32),
        C {
            n: i128,
        },
    }

    enum F {
        A = 1,
        B = 4,
        C = 9,
    }
}
