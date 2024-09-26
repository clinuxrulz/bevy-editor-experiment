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

pub struct FgrCtx {
    witness_created: bool,
    created_nodes: Vec<NodeRef>,
    witness_observe: bool,
    observed_nodes: Vec<NodeRef>,
    stack: Vec<NodeRef>,
    tmp_buffer_1: Vec<NodeRef>,
    tmp_buffer_2: Vec<NodeRef>,
    transaction_level: u32,
    defered_effects: Vec<Box<dyn FnOnce(&mut FgrCtx)>>,
}

impl FgrCtx {
    pub fn new() -> Self {
        Self {
            witness_created: false,
            created_nodes: Vec::new(),
            witness_observe: false,
            observed_nodes: Vec::new(),
            stack: Vec::new(),
            tmp_buffer_1: Vec::new(),
            tmp_buffer_2: Vec::new(),
            transaction_level: 0,
            defered_effects: Vec::new(),
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

    pub fn create_root<R, CALLBACK: FnOnce(&mut Self, RootScope) -> R>(&mut self, callback: CALLBACK) -> R {
        let scope = RootScope {
            scope: Rc::new(RefCell::new(Vec::new())),
        };
        self.witness_created = true;
        let result = callback(self, scope.clone());
        self.witness_created = false;
        {
            let mut scope = (*scope.scope).borrow_mut();
            for node in self.created_nodes.drain(..) {
                scope.push(node);
            }
        }
        result
    }

    pub fn create_effect<CALLBACK: FnMut(&mut FgrCtx) + 'static>(&mut self, callback: CALLBACK) {
        if !self.witness_created {
            panic!("Effect created outside of scope. Did you forget to call create_root()?");
        }
        self.batch(|fgr_ctx| {
            let impl_ = Rc::new(RefCell::new(EffectImpl {
                node_data: NodeData {
                    flag: NodeFlag::Stale,
                    changed: false,
                    dependencies: Vec::new(),
                    dependents: Vec::new(),
                    scoped: Vec::new(),
                },
                effect: Some(Rc::new(RefCell::new(callback))),
            }));
            let result = NodeRef {
                node: impl_,
            };
            fgr_ctx.created_nodes.push(result.clone());
            fgr_ctx.stack.push(result.clone());
            fgr_ctx.defered_effects.push(Box::new(move |fgr_ctx| {
                let mut impl_ = (*result.node).borrow_mut();
                impl_.update(fgr_ctx);
            }));
        });
    }
}

pub struct RootScope {
    scope: Rc<RefCell<Vec<NodeRef>>>,
}

impl Clone for RootScope {
    fn clone(&self) -> Self {
        Self {
            scope: Rc::clone(&self.scope),
        }
    }
}

impl RootScope {
    pub fn dispose(&mut self) {
        for node in (*self.scope).borrow_mut().drain(..) {
            (*node.node).borrow_mut().dispose();
        }
    }
}

pub struct EffectImpl {
    node_data: NodeData,
    effect: Option<Rc<RefCell<dyn FnMut(&mut FgrCtx)>>>,
}

impl IsNode for EffectImpl {
    fn is_source(&self) -> bool {
        false
    }

    fn node_data(&self) -> &NodeData {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData {
        &mut self.node_data
    }

    fn update(&mut self, fgr_ctx: &mut FgrCtx) -> bool {
        let Some(effect) = self.effect.as_ref() else { return false; };
        let effect = Rc::clone(effect);
        fgr_ctx.defered_effects.push(Box::new(move |fgr_ctx| {
            effect.borrow_mut()(fgr_ctx);
        }));
        false
    }

    fn dispose(&mut self) {
        //
        println!("dispose node {:?}", self as *const dyn IsNode);
        //
        let self2: *const dyn IsNode = self;
        for dependency in self.node_data.dependencies.drain(..) {
            dependency.with_node_mut(|node| {
                node.node_data_mut().dependents.retain(|x| {
                    let x2 = x.node.as_ptr() as *const dyn IsNode;
                    return std::ptr::addr_eq(self2, x2);
                });
            });
        }
        self.effect = None;
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
    pub fn new(fgr_ctx: &mut FgrCtx, update_fn: impl FnMut(&mut FgrCtx) -> A + 'static) -> Self
    where A: PartialEq<A>
    {
        Self::new_with_diff(fgr_ctx, update_fn, |a, b| a == b)        
    }

    pub fn new_no_diff(fgr_ctx: &mut FgrCtx, update_fn: impl FnMut(&mut FgrCtx) -> A + 'static) -> Self {
        Self::new_with_diff(fgr_ctx, update_fn, |_a, _b| false)
    }

    pub fn new_with_diff(fgr_ctx: &mut FgrCtx, mut update_fn: impl FnMut(&mut FgrCtx) -> A + 'static, compare_fn: impl FnMut(&A, &A) -> bool + 'static) -> Self {
        if !fgr_ctx.witness_created {
            panic!("Memo created outside of scope. Did you forget to call create_root()?");
        }
        let impl_ = Rc::new(RefCell::new(MemoImpl {
            node_data: NodeData {
                flag: NodeFlag::Ready,
                changed: false,
                dependencies: Vec::new(),
                dependents: Vec::new(),
                scoped: Vec::new(),
            },
            value: None,
            update_fn: None,
            compare_fn: Box::new(compare_fn),
        }));
        let result = Self {
            impl_,
        };
        let self_ref: NodeRef = (&result).into();
        fgr_ctx.witness_observe = true;
        let value = update_fn(fgr_ctx);
        fgr_ctx.witness_observe = false;
        for node in fgr_ctx.observed_nodes.drain(..) {
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
        fgr_ctx.created_nodes.push(self_ref);
        result
    }
}

impl<A: 'static> Memo<A> {
    pub fn value<'a>(&'a self, fgr_ctx: &mut FgrCtx) -> Ref<'_, A> {
        if fgr_ctx.witness_observe {
            fgr_ctx.observed_nodes.push(self.into());
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
    update_fn: Option<Box<dyn FnMut(&mut FgrCtx) -> A>>, // <-- only temporarly None during initialization.
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

    fn update(&mut self, fgr_ctx: &mut FgrCtx) -> bool {
        let next_value = (self.update_fn.as_mut().unwrap())(fgr_ctx);
        let changed = !(self.compare_fn)(&next_value, self.value.as_ref().unwrap());
        self.value = Some(next_value);
        changed
    }

    fn dispose(&mut self) {
        //
        println!("dispose node {:?}", self as *const dyn IsNode);
        //
        let self2: *const dyn IsNode = self;
        for dependency in self.node_data.dependencies.drain(..) {
            dependency.with_node_mut(|node| {
                node.node_data_mut().dependents.retain(|x| {
                    let x2 = x.node.as_ptr() as *const dyn IsNode;
                    return std::ptr::addr_eq(self2, x2);
                });
            });
        }
        for dependent in self.node_data.dependents.drain(..) {
            dependent.with_node_mut(|node| {
                node.node_data_mut().dependencies.retain(|x| {
                    let x2 = x.node.as_ptr() as *const dyn IsNode;
                    return std::ptr::addr_eq(self2, x2);
                });
            });
        }
        self.update_fn = None;
        for node in self.node_data.scoped.drain(..) {
            node.with_node_mut(|node| {
                node.dispose();
            });
        }
    }
}

pub struct Signal<A> {
    impl_: Rc<RefCell<SignalImpl<A>>>,
}

impl<A> Signal<A> {
    pub fn new(_fgr_ctx: &mut FgrCtx, value: A) -> Self {
        Self {
            impl_: Rc::new(RefCell::new(SignalImpl {
                node_data: NodeData {
                    flag: NodeFlag::Ready,
                    changed: false,
                    dependencies: Vec::new(),
                    dependents: Vec::new(),
                    scoped: Vec::new(),
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

    fn update(&mut self, _fgr_ctx: &mut FgrCtx) -> bool {
        let result = self.value_changed;
        self.value_changed = false;
        result
    }

    fn dispose(&mut self) { /* do nothing */}
}

impl<A: 'static> Signal<A> {
    pub fn value<'a>(&'a self, fgr_ctx: &mut FgrCtx) -> Ref<'_, A> {
        if fgr_ctx.witness_observe {
            fgr_ctx.observed_nodes.push(self.into());
        }
        let impl_ = (*self.impl_).borrow();
        let val = Ref::map(impl_, |impl_| &impl_.value);
        val
    }

    pub fn update_value<CALLBACK: FnOnce(&mut A)>(&mut self, fgr_ctx: &mut FgrCtx, callback: CALLBACK) {
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
    scoped: Vec<NodeRef>,
}

trait IsNode {
    fn is_source(&self) -> bool;
    fn node_data(&self) -> &NodeData;
    fn node_data_mut(&mut self) -> &mut NodeData;
    fn update(&mut self, fgr_ctx: &mut FgrCtx) -> bool;
    fn dispose(&mut self);
}

pub struct NodeRef {
    node: Rc<RefCell<dyn IsNode>>,
}

impl std::fmt::Debug for NodeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(NodeRef ")?;
        RefCell::as_ptr(&self.node).fmt(f)?;
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
        f(&*(*self.node).borrow())
    }
    fn with_node_mut<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut dyn IsNode) -> T,
    {
        f(&mut *(*self.node).borrow_mut())
    }
}

fn update_graph(fgr_ctx: &mut FgrCtx) {
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
                    println!("  update node {:?}", node);
                    let changed = node.with_node_mut(|n| {
                        if !is_source {
                            fgr_ctx.witness_created = true;
                            fgr_ctx.witness_observe = true;
                            for scoped in n.node_data_mut().scoped.drain(..) {
                                (*scoped.node).borrow_mut().dispose();
                            }
                        }
                        let changed = n.update(fgr_ctx);
                        if !is_source {
                            fgr_ctx.witness_observe = false;
                            fgr_ctx.witness_created = false;
                            let mut dependencies_to_remove: Vec<NodeRef> = Vec::new();
                            let mut dependencies_to_add: Vec<NodeRef> = Vec::new();
                            for dep in &fgr_ctx.observed_nodes {
                                let has = n.node_data().dependencies.contains(dep);
                                if !has {
                                    dependencies_to_add.push(dep.clone());
                                }
                            }
                            for dep in &n.node_data().dependencies {
                                let has = fgr_ctx.observed_nodes.contains(dep);
                                if !has {
                                    dependencies_to_remove.push(dep.clone());
                                }
                            }
                            for dep in dependencies_to_remove {
                                n.node_data_mut().dependencies.retain(|x| *x != dep);
                            }
                            for dep in dependencies_to_add {
                                n.node_data_mut().dependencies.push(dep);
                            }
                            std::mem::swap(&mut n.node_data_mut().scoped, &mut fgr_ctx.created_nodes);
                        }
                        let n2 = n.node_data_mut();
                        n2.changed = changed;
                        println!("  changed = {}", changed);
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
    let mut deferred_effects = Vec::new();
    std::mem::swap(&mut fgr_ctx.defered_effects, &mut deferred_effects);
    for effect in deferred_effects {
        effect(fgr_ctx);
    }
}

fn propergate_dependents_flags_to_stale(fgr_ctx: &mut FgrCtx) {
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
