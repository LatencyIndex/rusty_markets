/// ```
/// use rusty_markets::lsl::slice::sort_descending;
/// let mut x = [1, 3, 2];
/// sort_descending(&mut x);
/// assert_eq!(x, [3, 2, 1]);
/// ```
pub fn sort_descending<T: Ord>(x: &mut [T]) {
    x.sort_by(|a, b| a.cmp(b).reverse())
}
