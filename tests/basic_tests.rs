extern crate compact_vecmap;

use compact_vecmap::VecMap;

macro_rules! assert_seq_eq {
    ($iter:expr, [$($value:expr),*]) => {{
        let mut iter = $iter;
        $(
            assert_eq!(iter.next(), Some($value));
        )*
        assert_eq!(iter.next(), None);
    }};
    ($iter:expr, [$($value:expr,)*]) => {
        assert_seq_eq!($iter, [$($value),*])
    };
}

#[test]
fn insert_values_and_iter() {
    let mut map = VecMap::new();

    assert_eq!(map.insert(0, "Hello!"), None);
    assert_eq!(map.insert(1, "This is a map!"), None);
    assert_eq!(map.insert(5, "This index is further in~"), None);

    assert_seq_eq!(
        map.iter(),
        [
            (0, &"Hello!"),
            (1, &"This is a map!"),
            (5, &"This index is further in~"),
        ]
    );
}

#[test]
fn values_mut() {
    let mut map = VecMap::new();

    map.add(1);
    map.add(2);
    map.add(3);

    for value in map.values_mut() {
        *value *= 100;
    }

    assert_seq_eq!(map.iter(), [(0, &100), (1, &200), (2, &300)]);

    map.remove(1);

    assert_seq_eq!(map.iter(), [(0, &100), (2, &300)]);
}

#[test]
fn split_off() {
    let mut map = VecMap::new();

    for i in 1..=10 {
        map.add(i);
    }

    let map2 = map.split_off(5);

    assert_seq_eq!(map.values().cloned(), [1, 2, 3, 4, 5]);
    assert_seq_eq!(map2.values().cloned(), [6, 7, 8, 9, 10]);
}
