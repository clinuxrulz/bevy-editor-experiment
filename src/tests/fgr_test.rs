use crate::{cloned, fgr::*};

#[test]
fn test_fgr() {
    let mut fgr_ctx = FrgCtx::new();
    let fgr_ctx = &mut fgr_ctx;
    let mut sa = Signal::new(fgr_ctx, 1);
    let memo = Memo::new(fgr_ctx, cloned!((sa) => move |fgr_ctx| {
        let next = *sa.value(fgr_ctx) * 2;
        println!("next: {}", next);
        next
    }));
    sa.update_value(fgr_ctx, |v| *v += 1);
}
