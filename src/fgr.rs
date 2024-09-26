use std::borrow::Borrow;
use std::rc::Rc;
use std::cell::{Ref, RefCell, RefMut};

#[macro_export]
macro_rules! cloned {
    (($($arg:ident),*) => $e:expr) => {{
        // clone all the args
        $( let $arg = ::std::clone::Clone::clone(&$arg); )*
        $e
    }};
}

pub struct FrgCtx {
    observe_witness: bool,
    witness: Vec<NodeRef>,
    stack: Vec<NodeRef>,
    tmp_buffer_1: Vec<NodeRef>,
    tmp_buffer_2: Vec<NodeRef>,
    transaction_level: u32,
}

impl FrgCtx {
    pub fn new() -> Self {
        Self {
            observe_witness: false,
            witness: Vec::new(),
            stack: Vec::new(),
            tmp_buffer_1: Vec::new(),
            tmp_buffer_2: Vec::new(),
            transaction_level: 0,
        }
    }

    pub fn batch<R, CALLBACK: FnOnce(&mut Self) -> R>(&mut self, callback: CALLBACK) -> R {
        self.transaction_level += 1;
        let result = callback(self);
        self.transaction_level -= 1;
        if self.transaction_level == 0 {
            update_graph(self);
        }
        result
    }
}

pub struct Memo<A> {
    impl_: Rc<RefCell<MemoImpl<A>>>,
}

impl<A: 'static> std::fmt::Debug for Memo<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(Memo ")?;
        Into::<NodeRef>::into(&*self).fmt(f)?;
        f.write_str(")")
    }
}

impl<A: 'static> Memo<A> {
    pub fn new(fgr_ctx: &mut FrgCtx, update_fn: impl FnMut(&mut FrgCtx) -> A + 'static) -> Self
    where A: PartialEq<A>
    {
        Self::new_with_diff(fgr_ctx, update_fn, |a, b| a == b)        
    }

    pub fn new_no_diff(fgr_ctx: &mut FrgCtx, update_fn: impl FnMut(&mut FrgCtx) -> A + 'static) -> Self {
        Self::new_with_diff(fgr_ctx, update_fn, |_a, _b| false)
    }

    pub fn new_with_diff(fgr_ctx: &mut FrgCtx, mut update_fn: impl FnMut(&mut FrgCtx) -> A + 'static, compare_fn: impl FnMut(&A, &A) -> bool + 'static) -> Self {
        let impl_ = Rc::new(RefCell::new(MemoImpl {
            node_data: NodeData {
                flag: NodeFlag::Ready,
                changed: false,
                dependencies: Vec::new(),
                dependents: Vec::new(),
            },
            value: None,
            update_fn: None,
            compare_fn: Box::new(compare_fn),
        }));
        let result = Self {
            impl_,
        };
        let self_ref: NodeRef = (&result).into();
        fgr_ctx.observe_witness = true;
        let value = update_fn(fgr_ctx);
        fgr_ctx.observe_witness = false;
        for node in fgr_ctx.witness.drain(..) {
            node.with_node_mut(|node| {
                if !node.node_data_mut().dependents.contains(&self_ref) {
                    node.node_data_mut().dependents.push(self_ref.clone());
                }
            });
            self_ref.with_node_mut(|self_node| {
                if !self_node.node_data_mut().dependencies.contains(&node) {
                    self_node.node_data_mut().dependencies.push(node.clone());
                }
            });
        }
        {
            let mut tmp: RefMut<MemoImpl<A>> = (&*result.impl_).borrow_mut();
            tmp.value = Some(value);
            tmp.update_fn = Some(Box::new(update_fn));
        }
        result
    }
}

impl<A: 'static> Memo<A> {
    pub fn value<'a>(&'a self, fgr_ctx: &mut FrgCtx) -> Ref<'_, A> {
        if fgr_ctx.observe_witness {
            fgr_ctx.witness.push(self.into());
        }
        let impl_ = (*self.impl_).borrow();
        let val = Ref::map(impl_, |impl_| impl_.value.as_ref().unwrap());
        val
    }
}

impl<A> Clone for Memo<A> {
    fn clone(&self) -> Self {
        Self {
            impl_: Rc::clone(&self.impl_),
        }
    }
}

impl<A: 'static> Into<NodeRef> for &Memo<A> {
    fn into(self) -> NodeRef {
        let node = Rc::clone(&self.impl_);
        NodeRef {
            node,
        }
    }
}

pub struct MemoImpl<A> {
    node_data: NodeData,
    value: Option<A>, // <-- only temporarly None during initialization.
    update_fn: Option<Box<dyn FnMut(&mut FrgCtx) -> A>>, // <-- only temporarly None during initialization.
    compare_fn: Box<dyn FnMut(&A, &A) -> bool>,
}

impl<A> IsNode for MemoImpl<A> {
    fn is_source(&self) -> bool {
        false
    }

    fn node_data(&self) -> &NodeData {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData {
        &mut self.node_data
    }

    fn update(&mut self, fgr_ctx: &mut FrgCtx) -> bool {
        let next_value = (self.update_fn.as_mut().unwrap())(fgr_ctx);
        let changed = (self.compare_fn)(&next_value, self.value.as_ref().unwrap());
        self.value = Some(next_value);
        changed
    }
}

pub struct Signal<A> {
    impl_: Rc<RefCell<SignalImpl<A>>>,
}

impl<A> Signal<A> {
    pub fn new(_fgr_ctx: &mut FrgCtx, value: A) -> Self {
        Self {
            impl_: Rc::new(RefCell::new(SignalImpl {
                node_data: NodeData {
                    flag: NodeFlag::Ready,
                    changed: false,
                    dependencies: Vec::new(),
                    dependents: Vec::new(),
                },
                value,
                value_changed: false,
            })),
        }
    }
}

impl<A: 'static> std::fmt::Debug for Signal<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(Signal ")?;
        Into::<NodeRef>::into(&*self).fmt(f)?;
        f.write_str(")")
    }
}

impl<A> Clone for Signal<A> {
    fn clone(&self) -> Self {
        Self {
            impl_: Rc::clone(&self.impl_),
        }
    }
}

impl<A: 'static> Into<NodeRef> for &Signal<A> {
    fn into(self) -> NodeRef {
        let node = Rc::clone(&self.impl_);
        NodeRef {
            node,
        }
    }
}

struct SignalImpl<A> {
    node_data: NodeData,
    value: A,
    value_changed: bool,
}

impl<A> IsNode for SignalImpl<A> {
    fn is_source(&self) -> bool {
        true
    }

    fn node_data(&self) -> &NodeData {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData {
        &mut self.node_data
    }

    fn update(&mut self, _fgr_ctx: &mut FrgCtx) -> bool {
        let result = self.value_changed;
        self.value_changed = false;
        result
    }
}

impl<A: 'static> Signal<A> {
    pub fn value<'a>(&'a self, fgr_ctx: &mut FrgCtx) -> Ref<'_, A> {
        if fgr_ctx.observe_witness {
            fgr_ctx.witness.push(self.into());
        }
        let impl_ = (*self.impl_).borrow();
        let val = Ref::map(impl_, |impl_| &impl_.value);
        val
    }

    pub fn update_value<CALLBACK: FnOnce(&mut A)>(&mut self, fgr_ctx: &mut FrgCtx, callback: CALLBACK) {
        //
        println!("Signal::update_value called on {:?}", (Into::<NodeRef>::into(&*self)));
        //
        fgr_ctx.batch(|fgr_ctx| {
            {
                let mut impl_ = (*self.impl_).borrow_mut();
                callback(&mut impl_.value);
                impl_.value_changed = true;
                impl_.node_data.flag = NodeFlag::Stale;
            }
            // add self to stack for propergating dependent flags to stale.
            fgr_ctx.stack.push((&*self).into());
            propergate_dependents_flags_to_stale(fgr_ctx);
        });
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum NodeFlag {
    Ready,
    Stale,
}

struct NodeData {
    flag: NodeFlag,
    changed: bool,
    dependencies: Vec<NodeRef>,
    dependents: Vec<NodeRef>,
}

trait IsNode {
    fn is_source(&self) -> bool;
    fn node_data(&self) -> &NodeData;
    fn node_data_mut(&mut self) -> &mut NodeData;
    fn update(&mut self, fgr_ctx: &mut FrgCtx) -> bool;
}

pub struct NodeRef {
    node: Rc<RefCell<dyn IsNode>>,
}

impl std::fmt::Debug for NodeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(NodeRef ")?;
        Rc::as_ptr(&self.node).fmt(f)?;
        f.write_str(")")
    }
}

impl PartialEq for NodeRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.node, &other.node)
    }
}

impl Clone for NodeRef {
    fn clone(&self) -> Self {
        Self {
            node: Rc::clone(&self.node),
        }
    }
}

impl NodeRef {
    fn with_node<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&dyn IsNode) -> T,
    {
        let node: &RefCell<dyn IsNode> = self.node.borrow();
        f(&*node.borrow())
    }
    fn with_node_mut<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut dyn IsNode) -> T,
    {
        let node: &RefCell<dyn IsNode> = self.node.borrow();
        f(&mut *node.borrow_mut())
    }
}

fn update_graph(fgr_ctx: &mut FrgCtx) {
    let mut tmp_buffer_1 = Vec::new();
    let mut tmp_buffer_2 = Vec::new();
    let mut stack = Vec::new();
    std::mem::swap(&mut fgr_ctx.tmp_buffer_1, &mut tmp_buffer_1);
    std::mem::swap(&mut fgr_ctx.tmp_buffer_2, &mut tmp_buffer_2);
    std::mem::swap(&mut fgr_ctx.stack, &mut stack);
    //
    println!("update_graph: {} nodes", stack.len());
    //
    loop {
        let Some(node) = stack.pop() else { break; };
        //
        println!("at node {:?}", node);
        //
        let flag = node.with_node(|n| n.node_data().flag);
        println!("  flag: {:?}", flag);
        match flag {
            NodeFlag::Ready => { /* do nothing */ },
            NodeFlag::Stale => {
                let is_source = node.with_node(|n| n.is_source());
                node.with_node(|n| {
                    for dep in &n.node_data().dependencies {
                        tmp_buffer_2.push(dep.clone());
                    }
                });
                let mut any_dependencies_changed = false;
                let mut has_stale_dependencies = false;
                for dep in &*tmp_buffer_2 {
                    let flag = dep.with_node(|n| n.node_data().flag);
                    match flag {
                        NodeFlag::Ready => {
                            let changed = dep.with_node(|n| n.node_data().changed);
                            if changed {
                                any_dependencies_changed = true;
                                break;
                            }
                        },
                        NodeFlag::Stale => {
                            has_stale_dependencies = true;
                            stack.push(dep.clone());
                            break;
                        },
                    }
                }
                tmp_buffer_2.clear();
                if !has_stale_dependencies && (any_dependencies_changed || is_source) {
                    let changed = node.with_node_mut(|n| {
                        let changed = n.update(fgr_ctx);
                        let n2 = n.node_data_mut();
                        n2.changed = changed;
                        n2.flag = NodeFlag::Ready;
                        for dep in &n.node_data().dependents {
                            stack.push(dep.clone());
                        }
                        return changed;
                    });
                    if changed {
                        tmp_buffer_1.push(node);
                    }
                }
            },
        }
    }
    for node in tmp_buffer_1.drain(..) {
        node.with_node_mut(|n| {
            let n2 = n.node_data_mut();
            n2.changed = false;
        });
    }
    //
    println!("update_graph finished.");
    //
}

fn propergate_dependents_flags_to_stale(fgr_ctx: &mut FrgCtx) {
    loop {
        let Some(at) = fgr_ctx.stack.pop() else { break; };
        at.with_node(|n| {
            for dep in &n.node_data().dependents {
                fgr_ctx.tmp_buffer_1.push(dep.clone());
                fgr_ctx.tmp_buffer_2.push(dep.clone());
            }
        });
        for dep in fgr_ctx.tmp_buffer_1.drain(..) {
            dep.with_node_mut(|n| n.node_data_mut().flag = NodeFlag::Stale);
            fgr_ctx.stack.push(dep);
        }
    }
    for dep in fgr_ctx.tmp_buffer_2.drain(..) {
        fgr_ctx.stack.push(dep);
    }
}
