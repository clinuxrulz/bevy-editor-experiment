use crate::{cloned, fgr::*};

#[test]
fn test_fgr() {
    let mut fgr_ctx = FrgCtx::new();
    let fgr_ctx = &mut fgr_ctx;
    let mut sa = Signal::new(1);
    let memo = Memo::new(cloned!((sa) => move || *sa.value() * 2));
    sa.update_value(fgr_ctx, |v| *v += 1);
}
