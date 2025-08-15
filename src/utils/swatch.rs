#[derive(Clone, Copy)]
pub struct Swatch<T> {
    pub xy: (i8, i8),
    pub op: T,
}
