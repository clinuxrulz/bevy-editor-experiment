use crate::{cloned, fgr::*};

#[test]
fn test_fgr() {
    let mut fgr_ctx = FgrCtx::new();
    let fgr_ctx = &mut fgr_ctx;
    let mut sa = Signal::new(fgr_ctx, 1);
    //
    println!("sa node: {:?}", sa);
    //
    let mut scope = fgr_ctx.create_root(|fgr_ctx, scope| {
        let memo_a = Memo::new(fgr_ctx, cloned!((sa) => move |fgr_ctx| {
            let next = *sa.value(fgr_ctx) * 2;
            println!("memoA: {}", next);
            next
        }));
        //
        println!("memo_a node: {:?}", memo_a);
        //
        let memo_b = Memo::new(fgr_ctx, cloned!((memo_a) => move |fgr_ctx| {
            let next = *memo_a.value(fgr_ctx) * 3;
            println!("memoB: {}", next);
            next
        }));
        //
        println!("memo_b node: {:?}", memo_b);
        //
        fgr_ctx.create_effect(cloned!((memo_a, memo_b) => move |fgr_ctx| {
            println!("effect (memo_a * memo_b): {}", *memo_a.value(fgr_ctx) * *memo_b.value(fgr_ctx));
        }));
        //
        scope
    });
    sa.update_value(fgr_ctx, |v| *v += 1);
    //scope.dispose();
}
