extern crate compact_vecmap;

use std::sync::atomic::{AtomicUsize, Ordering};

use compact_vecmap::VecMap;

#[test]
fn drain() {
    static DEALLOCS: AtomicUsize = AtomicUsize::new(0);
    struct Foo(u64);
    const LIVE_VALUE: u64 = 0x123456789ABCDEF;
    const DEAD_VALUE: u64 = 0xFEDCBA987654321;
    impl Drop for Foo {
        fn drop(&mut self) {
            assert_eq!(self.0, LIVE_VALUE);
            self.0 = DEAD_VALUE;
            DEALLOCS.fetch_add(1, Ordering::SeqCst);
        }
    }

    DEALLOCS.store(0, Ordering::SeqCst);

    let mut map = VecMap::new();
    map.add(Foo(LIVE_VALUE));
    map.add(Foo(LIVE_VALUE));
    map.add(Foo(LIVE_VALUE));
    map.add(Foo(LIVE_VALUE));
    map.add(Foo(LIVE_VALUE));

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
