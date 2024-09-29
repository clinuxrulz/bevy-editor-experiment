use crate::{cloned, fgr::*};

#[test]
fn test_fgr() {
    struct Ctx {
        fgr_ctx: FgrCtx<Ctx>,
    }
    impl HasFgrCtx for Ctx {
        fn fgr_ctx<'a>(&'a mut self) -> impl std::ops::DerefMut<Target=FgrCtx<Ctx>> + 'a {
            &mut self.fgr_ctx
        }
    }
    let mut ctx = Ctx { fgr_ctx: FgrCtx::new() };
    let ctx = &mut ctx;
    //let mut fgr_ctx = FgrCtx::new();
    //let fgr_ctx = &mut fgr_ctx;
    let mut sa = Signal::new(ctx, 1);
    //
    println!("sa node: {:?}", sa);
    //
    let mut scope = ctx.fgr_create_root(|ctx, scope| {
        let memo_a = Memo::new(ctx, cloned!((sa) => move |ctx| {
            let next = *sa.value(ctx) * 2;
            println!("memoA: {}", next);
            next
        }));
        //
        println!("memo_a node: {:?}", memo_a);
        //
        let memo_b = Memo::new(ctx, cloned!((memo_a) => move |ctx| {
            let next = *memo_a.value(ctx) * 3;
            println!("memoB: {}", next);
            next
        }));
        //
        println!("memo_b node: {:?}", memo_b);
        //
        ctx.fgr_create_effect(cloned!((memo_a, memo_b) => move |ctx| {
            println!("effect (memo_a * memo_b): {}", *memo_a.value(ctx) * *memo_b.value(ctx));
        }));
        //
        scope
    });
    //
    print_graph((&sa).into());
    //
    sa.update_value(ctx, |v| *v += 1);
    scope.dispose(ctx);
}
