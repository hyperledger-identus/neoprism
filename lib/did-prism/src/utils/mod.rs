pub mod paging;

/// Check if the given slice contains unique items.
///
/// # Examples
/// ```
/// use identus_did_prism::utils::is_slice_unique;
/// assert_eq!(is_slice_unique(&[1, 2, 3]), true);
/// assert_eq!(is_slice_unique(&[1, 2, 2]), false);
/// assert_eq!(is_slice_unique(&[1, 1, 1]), false);
/// assert_eq!(is_slice_unique::<i32>(&[]), true);
/// ```
pub fn is_slice_unique<T>(items: &[T]) -> bool
where
    T: Eq + Ord,
{
    let mut set = std::collections::BTreeSet::new();
    items.iter().all(|x| set.insert(x))
}
/// Location of a particular point in the source code.
/// Intended to use for debugging purposes.
#[derive(Debug, Clone, derive_more::Display)]
#[display("[at {}:{}]", file, line)]
pub struct Location {
    pub file: &'static str,
    pub line: u32,
}
