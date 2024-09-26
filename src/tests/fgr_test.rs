use crate::{cloned, fgr::*};

#[test]
fn test_fgr() {
    let mut fgr_ctx = FrgCtx::new();
    let fgr_ctx = &mut fgr_ctx;
    let mut sa = Signal::new(fgr_ctx, 1);
    let memo_a = Memo::new(fgr_ctx, cloned!((sa) => move |fgr_ctx| {
        let next = *sa.value(fgr_ctx) * 2;
        println!("memoA: {}", next);
        next
    }));
    let memo_b = Memo::new(fgr_ctx, cloned!((memo_a) => move |fgr_ctx| {
        let next = *memo_a.value(fgr_ctx) * 3;
        println!("memoB: {}", next);
        next
    }));
    sa.update_value(fgr_ctx, |v| *v += 1);
}
