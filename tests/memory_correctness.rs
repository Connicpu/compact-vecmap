extern crate compact_vecmap;

use std::sync::atomic::{AtomicUsize, Ordering};

use compact_vecmap::VecMap;

#[test]
fn drain() {
    static DEALLOCS: AtomicUsize = AtomicUsize::new(0);
    struct Foo;
    impl Drop for Foo {
        fn drop(&mut self) {
            DEALLOCS.fetch_add(1, Ordering::SeqCst);
        }
    }

    DEALLOCS.store(0, Ordering::SeqCst);

    let mut map = VecMap::new();
    map.add(Foo);
    map.add(Foo);
    map.add(Foo);
    map.add(Foo);
    map.add(Foo);

    assert_eq!(DEALLOCS.load(Ordering::SeqCst), 0);

    {
        let mut iter = map.drain();
        for _ in iter.by_ref().take(2) {}

        assert_eq!(DEALLOCS.load(Ordering::SeqCst), 2);

        for _ in iter.by_ref().take(1) {}

        assert_eq!(DEALLOCS.load(Ordering::SeqCst), 3);

        drop(iter);

        assert_eq!(DEALLOCS.load(Ordering::SeqCst), 5);
    }

    drop(map);
    assert_eq!(DEALLOCS.load(Ordering::SeqCst), 5);
}
