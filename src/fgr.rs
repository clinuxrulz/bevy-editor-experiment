use std::{ops::DerefMut, sync::{Arc, RwLock, RwLockReadGuard}};

use bevy::prelude::{Resource, World};
use crate::cloned;

const DEBUG_LOG: bool = false;

#[derive(Resource)]
pub struct FgrCtx<CTX> {
    next_id: u64,
    witness_created: bool,
    created_nodes: Vec<NodeRef<CTX>>,
    witness_observe: bool,
    observed_nodes: Vec<NodeRef<CTX>>,
    update_flag_signal: Option<Signal<CTX,u32>>,
    stack: Vec<NodeRef<CTX>>,
    tmp_buffer_1: Vec<NodeRef<CTX>>,
    tmp_buffer_2: Vec<NodeRef<CTX>>,
    transaction_level: u32,
    defered_effects: Vec<Box<dyn FnOnce(&mut CTX) + Sync + Send>>,
}

pub trait HasFgrCtx where Self: Sized {
    fn fgr_ctx<'a>(&'a mut self) -> impl DerefMut<Target=FgrCtx<Self>> + 'a;
}

impl HasFgrCtx for World {
    fn fgr_ctx<'a>(&'a mut self) -> impl DerefMut<Target=FgrCtx<Self>> + 'a {
        self.get_resource_mut::<FgrCtx<Self>>().unwrap()
    }
}

pub trait FgrExtensionMethods {
    fn fgr_untrack<R, CALLBACK: FnOnce(&mut Self) -> R>(&mut self, callback: CALLBACK) -> R;
    fn fgr_batch<R, CALLBACK: FnOnce(&mut Self) -> R>(&mut self, callback: CALLBACK) -> R;
    fn fgr_create_root<R, CALLBACK: FnOnce(&mut Self, RootScope<Self>) -> R>(&mut self, callback: CALLBACK) -> R;
    fn fgr_create_effect<CALLBACK: FnMut(&mut Self) + Send + Sync + 'static>(&mut self, callback: CALLBACK);
    fn fgr_on_cleanup<CALLBACK: FnMut(&mut Self) + Send + Sync + 'static>(&mut self, callback: CALLBACK);
    fn fgr_on_update<CALLBACK: FnMut(&mut Self) + Send + Sync + 'static>(&mut self, callback: CALLBACK);
    fn fgr_update(&mut self);

    fn fgr_on_mount<CALLBACK: FnOnce(&mut Self) + Send + Sync + 'static>(&mut self, callback: CALLBACK) where Self: HasFgrCtx + Send + Sync + 'static {
        // must happen after the layout manager of bevy ui has run
        let first = Signal::new(self, true);
        let mut callback2 = Some(callback);
        Memo::new_no_diff(self, move |ctx| {
            if !*first.value(ctx) {
                return;
            }
            let mut callback3 = None;
            std::mem::swap(&mut callback3, &mut callback2);
            ctx.fgr_on_update(cloned!((first) => move |ctx| {
                let mut callback4 = None;
                std::mem::swap(&mut callback4, &mut callback3);
                let Some(callback5) = callback4 else { return; };
                callback5(ctx);
                first.update_value(ctx, |x| *x = false);
            }));
        });
    }
}

impl<CTX: HasFgrCtx + 'static> FgrExtensionMethods for CTX {
    fn fgr_untrack<R, CALLBACK: FnOnce(&mut Self) -> R>(&mut self, callback: CALLBACK) -> R {
        FgrCtx::untrack(self, callback)
    }

    fn fgr_batch<R, CALLBACK: FnOnce(&mut Self) -> R>(&mut self, callback: CALLBACK) -> R {
        FgrCtx::batch(self, callback)
    }

    fn fgr_create_root<R, CALLBACK: FnOnce(&mut Self, RootScope<Self>) -> R>(&mut self, callback: CALLBACK) -> R {
        FgrCtx::create_root(self, callback)
    }

    fn fgr_create_effect<CALLBACK: FnMut(&mut Self) + Send + Sync + 'static>(&mut self, callback: CALLBACK) {
        FgrCtx::create_effect(self, callback)
    }

    fn fgr_on_cleanup<CALLBACK: FnMut(&mut Self) + Send + Sync + 'static>(&mut self, callback: CALLBACK) {
        FgrCtx::on_cleanup(self, callback)
    }

    fn fgr_on_update<CALLBACK: FnMut(&mut Self) + Send + Sync + 'static>(&mut self, callback: CALLBACK) {
        FgrCtx::on_update(self, callback)
    }

    fn fgr_update(&mut self) {
        FgrCtx::update(self);
    }
}

impl<CTX: HasFgrCtx + 'static> FgrCtx<CTX> {
    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn track_observed<R, CALLBACK: FnOnce()->R>(&mut self, callback: CALLBACK) -> (Vec<NodeRef<CTX>>, R) {
        let mut tmp = Vec::new();
        std::mem::swap(&mut tmp, &mut self.observed_nodes);
        let r = callback();
        std::mem::swap(&mut tmp, &mut self.observed_nodes);
        (tmp, r)
    }

    fn track_created<R, CALLBACK: FnOnce()->R>(&mut self, callback: CALLBACK) -> (Vec<NodeRef<CTX>>, R) {
        let mut tmp = Vec::new();
        std::mem::swap(&mut tmp, &mut self.created_nodes);
        let r = callback();
        std::mem::swap(&mut tmp, &mut self.created_nodes);
        (tmp, r)
    }

    pub fn new() -> Self {
        Self {
            next_id: 0,
            witness_created: false,
            created_nodes: Vec::new(),
            witness_observe: false,
            observed_nodes: Vec::new(),
            update_flag_signal: None,
            stack: Vec::new(),
            tmp_buffer_1: Vec::new(),
            tmp_buffer_2: Vec::new(),
            transaction_level: 0,
            defered_effects: Vec::new(),
        }
    }

    pub fn untrack<R, CALLBACK: FnOnce(&mut CTX) -> R>(ctx: &mut CTX, callback: CALLBACK) -> R {
        let mut tmp = Vec::new();
        std::mem::swap(&mut ctx.fgr_ctx().observed_nodes, &mut tmp);
        let result = callback(ctx);
        std::mem::swap(&mut ctx.fgr_ctx().observed_nodes, &mut tmp);
        result
    }

    pub fn batch<R, CALLBACK: FnOnce(&mut CTX) -> R>(ctx: &mut CTX, callback: CALLBACK) -> R {
        {
            let fgr_ctx = &mut ctx.fgr_ctx();
            fgr_ctx.transaction_level += 1;
        }
        let result = callback(ctx);
        let hit_level_zero: bool;
        {
            let fgr_ctx = &mut ctx.fgr_ctx();
            fgr_ctx.transaction_level -= 1;
            hit_level_zero = fgr_ctx.transaction_level == 0;
        }
        if hit_level_zero {
            update_graph(ctx);
        }
        result
    }

    pub fn create_root<R, CALLBACK: FnOnce(&mut CTX, RootScope<CTX>) -> R>(ctx: &mut CTX, callback: CALLBACK) -> R {
        ctx.fgr_batch(|ctx| {
            let scope = RootScope {
                scope: Arc::new(RwLock::new(Vec::new())),
            };
            ctx.fgr_ctx().witness_created = true;
            let result = callback(ctx, scope.clone());
            ctx.fgr_ctx().witness_created = false;
            {
                let mut scope = (*scope.scope).write().unwrap();
                for node in ctx.fgr_ctx().created_nodes.drain(..) {
                    scope.push(node);
                }
            }
            result
        })
    }

    pub fn create_effect<CALLBACK: FnMut(&mut CTX) + Send + Sync + 'static>(ctx: &mut CTX, callback: CALLBACK) {
        if !ctx.fgr_ctx().witness_created {
            panic!("Effect created outside of scope. Did you forget to call create_root()?");
        }
        let id = ctx.fgr_ctx().alloc_id();
        let effect: Arc<RwLock<dyn FnMut(&mut CTX) + Send + Sync>> = Arc::new(RwLock::new(callback));
        let impl_: Arc<RwLock<dyn IsNode<CTX> + Send + Sync>> = Arc::new(RwLock::new(EffectImpl {
            node_data: NodeData {
                id,
                flag: NodeFlag::Stale,
                changed: false,
                dependencies: Vec::new(),
                dependents: Vec::new(),
                scoped: Vec::new(),
            },
            effect: Some(Arc::clone(&effect)),
        }));
        let result = NodeRef {
            id,
            node: Arc::clone(&impl_),
        };
        ctx.fgr_ctx().created_nodes.push(result.clone());
        ctx.fgr_ctx().stack.push(result.clone());
        ctx.fgr_ctx().defered_effects.push(Box::new(move |ctx| {
            ctx.fgr_ctx().witness_created = true;
            ctx.fgr_ctx().witness_observe = true;
            (*effect).write().unwrap()(ctx);
            ctx.fgr_ctx().witness_created = false;
            ctx.fgr_ctx().witness_observe = false;
            let mut impl_ = impl_.write().unwrap();
            for node in ctx.fgr_ctx().observed_nodes.drain(..) {
                impl_.node_data_mut().dependencies.push(node.clone());
                node.with_node_mut(|node| {
                    node.node_data_mut().dependents.push(result.clone());
                });
            }
            for node in ctx.fgr_ctx().created_nodes.drain(..) {
                impl_.node_data_mut().scoped.push(node.clone());
            }
        }));
    }

    pub fn on_cleanup(ctx: &mut CTX, callback: impl FnMut(&mut CTX) + Send + Sync + 'static) {
        if !ctx.fgr_ctx().witness_created {
            panic!("on_cleanup created outside of scope. Did you forget to call create_root()?");
        }
        let id = ctx.fgr_ctx().alloc_id();
        let impl_: Arc<RwLock<dyn IsNode<CTX> + Send + Sync>> = Arc::new(RwLock::new(CleanupImpl {
            node_data: NodeData {
                id,
                flag: NodeFlag::Stale,
                changed: false,
                dependencies: Vec::new(),
                dependents: Vec::new(),
                scoped: Vec::new(),
            },
            cleanup: Some(Arc::new(RwLock::new(callback))),
        }));
        let node = NodeRef {
            id,
            node: Arc::clone(&impl_),
        };
        ctx.fgr_ctx().created_nodes.push(node.clone());
    }

    pub fn on_update(ctx: &mut CTX, mut callback: impl FnMut(&mut CTX) + Send + Sync + 'static) {
        if !ctx.fgr_ctx().witness_created {
            panic!("on_update created outside of scope. Did you forget to call create_root()?");
        }
        let update_flag_signal: Option<Signal<CTX, u32>> = ctx.fgr_ctx().update_flag_signal.clone();
        let update_flag_signal_2: Signal<CTX, u32>;
        if let Some(x) = &update_flag_signal {
            update_flag_signal_2 = x.clone();
        } else {
            update_flag_signal_2 = Signal::new(ctx, 0);
            ctx.fgr_ctx().update_flag_signal = Some(update_flag_signal_2.clone());
        }
        FgrCtx::create_effect(ctx, move |ctx: &mut CTX| {
            let _ = *update_flag_signal_2.value(ctx);
            FgrCtx::untrack(ctx, |ctx| callback(ctx));
        });
    }

    pub fn update(ctx: &mut CTX) {
        let Some(mut update_flag_signal) = ctx.fgr_ctx().update_flag_signal.clone() else { return; };
        update_flag_signal.update_value(ctx, |x| *x = 1 - *x);
    }
}

#[derive(Resource)]
pub struct RootScope<CTX> {
    scope: Arc<RwLock<Vec<NodeRef<CTX>>>>,
}

impl<CTX> Clone for RootScope<CTX> {
    fn clone(&self) -> Self {
        Self {
            scope: Arc::clone(&self.scope),
        }
    }
}

impl<CTX: HasFgrCtx> RootScope<CTX> {
    pub fn dispose(&mut self, ctx: &mut CTX) {
        let mut scope: Vec<NodeRef<CTX>> = Vec::new();
        std::mem::swap(&mut *(*self.scope).write().unwrap(), &mut scope);
        for node in scope {
            (*node.node).write().unwrap().dispose(ctx);
        }
    }
}

pub struct CleanupImpl<CTX> {
    node_data: NodeData<CTX>,
    cleanup: Option<Arc<RwLock<dyn FnMut(&mut CTX) + Send + Sync>>>,
}

impl<CTX: HasFgrCtx + 'static> IsNode<CTX> for CleanupImpl<CTX> {
    fn is_source(&self) -> bool {
        false
    }

    fn is_sink(&self) -> bool {
        false
    }

    fn node_data(&self) -> &NodeData<CTX> {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData<CTX> {
        &mut self.node_data
    }

    fn update(&mut self, _self_node_ref: NodeRef<CTX>, _ctx: &mut CTX) -> bool {
        false
    }

    fn dispose(&mut self, ctx: &mut CTX) {
        if let Some(cleanup) = self.cleanup.as_ref() {
            (*cleanup).write().unwrap()(ctx);
        }
    }
}

pub struct EffectImpl<CTX> {
    node_data: NodeData<CTX>,
    effect: Option<Arc<RwLock<dyn FnMut(&mut CTX) + Send + Sync>>>,
}

impl<CTX: HasFgrCtx + 'static> IsNode<CTX> for EffectImpl<CTX> {
    fn is_source(&self) -> bool {
        false
    }

    fn is_sink(&self) -> bool {
        true
    }

    fn node_data(&self) -> &NodeData<CTX> {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData<CTX> {
        &mut self.node_data
    }

    fn update(&mut self, self_node_ref: NodeRef<CTX>, ctx: &mut CTX) -> bool {
        let Some(effect) = self.effect.as_ref() else { return false; };
        let effect = Arc::clone(effect);
        ctx.fgr_ctx().defered_effects.push(Box::new(move |ctx| {
            ctx.fgr_ctx().witness_observe = true;
            ctx.fgr_ctx().witness_created = true;
            effect.write().unwrap()(ctx);
            ctx.fgr_ctx().witness_observe = false;
            ctx.fgr_ctx().witness_created = false;
            let mut dependencies_to_remove: Vec<NodeRef<CTX>> = Vec::new();
            let mut dependencies_to_add: Vec<NodeRef<CTX>> = Vec::new();
            for dep in &ctx.fgr_ctx().observed_nodes {
                let has = self_node_ref.with_node(|self_node| self_node.node_data().dependencies.contains(dep));
                if !has {
                    dependencies_to_add.push(dep.clone());
                }
            }
            self_node_ref.with_node(|self_node| {
                for dep in &self_node.node_data().dependencies {
                    let has = ctx.fgr_ctx().observed_nodes.contains(dep);
                    if !has {
                        dependencies_to_remove.push(dep.clone());
                    }
                }
            });
            for dep in dependencies_to_remove {
                self_node_ref.with_node_mut(|self_node| self_node.node_data_mut().dependencies.retain(|x| *x != dep));
            }
            for dep in dependencies_to_add {
                self_node_ref.with_node_mut(|self_node| self_node.node_data_mut().dependencies.push(dep));
            }
            self_node_ref.with_node_mut(|self_node| {
                std::mem::swap(&mut self_node.node_data_mut().scoped, &mut ctx.fgr_ctx().created_nodes);
            });
        }));
        false
    }

    fn dispose(&mut self, _ctx: &mut CTX) {
        //
        if DEBUG_LOG {
            println!("dispose node {:}", self.node_data.id);
        }
        //
        for dependency in self.node_data.dependencies.drain(..) {
            dependency.with_node_mut(|node| {
                node.node_data_mut().dependents.retain(|x| {
                    return x.id != self.node_data.id;
                });
            });
        }
        self.effect = None;
    }
}

pub struct BoxedAccessor<CTX, A>(Arc<dyn BoxedAccessorImpl<CTX, A> + Send + Sync>);

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Into<BoxedAccessor<CTX,A>> for Memo<CTX, A> {
    fn into(self) -> BoxedAccessor<CTX,A> {
        BoxedAccessor(Arc::new(self.clone()))
    }
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Into<BoxedAccessor<CTX,A>> for Signal<CTX, A> {
    fn into(self) -> BoxedAccessor<CTX,A> {
        BoxedAccessor(Arc::new(self.clone()))
    }
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Into<BoxedAccessor<CTX,A>> for ConstAccessor<A> {
    fn into(self) -> BoxedAccessor<CTX,A> {
        BoxedAccessor(Arc::new(self.clone()))
    }
}

impl<CTX, A> Clone for BoxedAccessor<CTX, A> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<CTX,A> BoxedAccessor<CTX,A> {
    pub fn with_value<'a, R:'static, CALLBACK: FnOnce(&A)->R + 'a>(&'a self, ctx: &mut CTX, callback: CALLBACK) -> R {
        let mut result: Option<R> = None;
        self.0.with_value(ctx, Box::new(|a| {
            result = Some(callback(a));
        }));
        return result.unwrap();
    }
}

trait BoxedAccessorImpl<CTX,A> {
    fn with_value<'a>(&'a self, ctx: &mut CTX, callback: Box<dyn FnOnce(&A) + 'a>);
}

impl <CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> BoxedAccessorImpl<CTX, A> for Memo<CTX, A> {
    fn with_value<'a>(&'a self, ctx: &mut CTX, callback: Box<dyn FnOnce(&A) + 'a>) {
        callback(&self.value(ctx));
    }
}

impl <CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> BoxedAccessorImpl<CTX, A> for Signal<CTX, A> {
    fn with_value<'a>(&'a self, ctx: &mut CTX, callback: Box<dyn FnOnce(&A) + 'a>) {
        callback(&self.value(ctx));
    }
}

impl <CTX,A> BoxedAccessorImpl<CTX,A> for ConstAccessor<A> {
    fn with_value<'a>(&'a self, _ctx: &mut CTX, callback: Box<dyn FnOnce(&A) + 'a>) {
        callback(&self.0);
    }
}

pub trait Accessor<CTX, A> {
    fn value<'a>(&'a self, ctx: &mut CTX) -> impl std::ops::Deref<Target=A> + 'a;
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Accessor<CTX, A> for Memo<CTX, A> {
    fn value<'a>(&'a self, ctx: &mut CTX) -> impl std::ops::Deref<Target=A> + 'a {
        self.value(ctx)
    }
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Accessor<CTX, A> for Signal<CTX, A> {
    fn value<'a>(&'a self, ctx: &mut CTX) -> impl std::ops::Deref<Target=A> + 'a {
        self.value(ctx)
    }
}

impl<CTX,A> Accessor<CTX,A> for ConstAccessor<A> {
    fn value<'a>(&'a self, _ctx: &mut CTX) -> impl std::ops::Deref<Target=A> + 'a {
        &*self.0
    }
}

impl<CTX,A:'static> Accessor<CTX,A> for BoxedAccessor<CTX,A> {
    fn value<'a>(&'a self, ctx: &mut CTX) -> impl std::ops::Deref<Target=A> + 'a {
        struct MyRef<A> {
            value: *const A,
        }
        impl<A> std::ops::Deref for MyRef<A> {
            type Target = A;
            fn deref(&self) -> &Self::Target {
                unsafe { &*self.value }
            }
        }
        let mut my_ref = MyRef { value: 0 as *const A };
        self.with_value(ctx, |a| {
            my_ref.value = a as *const A;
        });
        my_ref
    }
}

pub struct ConstAccessor<A>(Arc<A>);

impl<A> Clone for ConstAccessor<A> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<A> ConstAccessor<A> {
    pub fn new(value: A) -> Self {
        Self(Arc::new(value))
    }
}

pub struct Memo<CTX, A> {
    impl_: Arc<RwLock<MemoImpl<CTX, A>>>,
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> std::fmt::Debug for Memo<CTX, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Memo {})", Into::<NodeRef<CTX>>::into(self).id)
    }
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Memo<CTX, A> {
    pub fn new(fgr_ctx: &mut CTX, update_fn: impl FnMut(&mut CTX) -> A + Send + Sync + 'static) -> Self
    where A: PartialEq<A>
    {
        Self::new_with_diff(fgr_ctx, update_fn, |a, b| a == b)        
    }

    pub fn new_no_diff(fgr_ctx: &mut CTX, update_fn: impl FnMut(&mut CTX) -> A + Send + Sync + 'static) -> Self {
        Self::new_with_diff(fgr_ctx, update_fn, |_a, _b| false)
    }

    pub fn new_with_diff(ctx: &mut CTX, mut update_fn: impl FnMut(&mut CTX) -> A + Send + Sync + 'static, compare_fn: impl FnMut(&A, &A) -> bool + Send + Sync + 'static) -> Self {
        if !ctx.fgr_ctx().witness_created {
            panic!("Memo created outside of scope. Did you forget to call create_root()?");
        }
        let id = ctx.fgr_ctx().alloc_id();
        let impl_ = Arc::new(RwLock::new(MemoImpl {
            node_data: NodeData {
                id,
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
        let self_ref: NodeRef<CTX> = (&result).into();
        ctx.fgr_ctx().witness_observe = true;
        let value = update_fn(ctx);
        ctx.fgr_ctx().witness_observe = false;
        for node in ctx.fgr_ctx().observed_nodes.drain(..) {
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
            let mut tmp = (&*result.impl_).write().unwrap();
            tmp.value = Some(value);
            tmp.update_fn = Some(Box::new(update_fn));
        }
        ctx.fgr_ctx().created_nodes.push(self_ref);
        result
    }
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Memo<CTX, A> {
    pub fn value<'a>(&'a self, ctx: &mut CTX) -> impl std::ops::Deref<Target=A> + 'a {
        if ctx.fgr_ctx().witness_observe {
            ctx.fgr_ctx().observed_nodes.push(self.into());
        }
        let impl_ = (*self.impl_).read().unwrap();
        struct MyRef<'a,CTX,A> {
            impl_: RwLockReadGuard<'a, MemoImpl<CTX, A>>,
        }
        impl<'a, CTX, A> std::ops::Deref for MyRef<'a, CTX, A> {
            type Target = A;
            fn deref(&self) -> &Self::Target {
                self.impl_.value.as_ref().unwrap()
            }
        }
        let val = MyRef { impl_: impl_ };
        val
    }
}

impl<CTX, A> Clone for Memo<CTX, A> {
    fn clone(&self) -> Self {
        Self {
            impl_: Arc::clone(&self.impl_),
        }
    }
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Into<NodeRef<CTX>> for &Memo<CTX, A> {
    fn into(self) -> NodeRef<CTX> {
        let id = self.impl_.read().unwrap().node_data.id;
        let node = Arc::clone(&self.impl_);
        NodeRef {
            id,
            node,
        }
    }
}

pub struct MemoImpl<CTX, A> {
    node_data: NodeData<CTX>,
    value: Option<A>, // <-- only temporarly None during initialization.
    update_fn: Option<Box<dyn FnMut(&mut CTX) -> A + Send + Sync>>, // <-- only temporarly None during initialization.
    compare_fn: Box<dyn FnMut(&A, &A) -> bool + Send + Sync>,
}

impl<CTX: HasFgrCtx, A> IsNode<CTX> for MemoImpl<CTX,A> {
    fn is_source(&self) -> bool {
        false
    }

    fn is_sink(&self) -> bool {
        false
    }

    fn node_data(&self) -> &NodeData<CTX> {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData<CTX> {
        &mut self.node_data
    }

    fn update(&mut self, _self_node_ref: NodeRef<CTX>, ctx: &mut CTX) -> bool {
        let next_value = (self.update_fn.as_mut().unwrap())(ctx);
        let changed = !(self.compare_fn)(&next_value, self.value.as_ref().unwrap());
        self.value = Some(next_value);
        changed
    }

    fn dispose(&mut self, ctx: &mut CTX) {
        //
        if DEBUG_LOG {
            println!("dispose node {}", self.node_data.id);
        }
        //
        for dependency in self.node_data.dependencies.drain(..) {
            dependency.with_node_mut(|node| {
                node.node_data_mut().dependents.retain(|x| {
                    return x.id != self.node_data.id;
                });
            });
        }
        for dependent in self.node_data.dependents.drain(..) {
            dependent.with_node_mut(|node| {
                node.node_data_mut().dependencies.retain(|x| {
                    return x.id != self.node_data.id;
                });
            });
        }
        self.update_fn = None;
        for node in self.node_data.scoped.drain(..) {
            node.with_node_mut(|node| {
                node.dispose(ctx);
            });
        }
    }
}

pub struct Signal<CTX, A> {
    impl_: Arc<RwLock<SignalImpl<CTX, A>>>,
}

impl<CTX: HasFgrCtx + 'static, A> Signal<CTX, A> {
    pub fn new(ctx: &mut CTX, value: A) -> Self {
        let id = ctx.fgr_ctx().alloc_id();
        Self {
            impl_: Arc::new(RwLock::new(SignalImpl {
                node_data: NodeData {
                    id,
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

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> std::fmt::Debug for Signal<CTX, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Signal {})", Into::<NodeRef<CTX>>::into(self).id)
    }
}

impl<CTX, A> Clone for Signal<CTX, A> {
    fn clone(&self) -> Self {
        Self {
            impl_: Arc::clone(&self.impl_),
        }
    }
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Into<NodeRef<CTX>> for &Signal<CTX, A> {
    fn into(self) -> NodeRef<CTX> {
        let id = self.impl_.read().unwrap().node_data.id;
        let node = Arc::clone(&self.impl_);
        NodeRef {
            id,
            node,
        }
    }
}

struct SignalImpl<CTX, A> {
    node_data: NodeData<CTX>,
    value: A,
    value_changed: bool,
}

impl<CTX: HasFgrCtx + 'static, A> IsNode<CTX> for SignalImpl<CTX, A> {
    fn is_source(&self) -> bool {
        true
    }

    fn is_sink(&self) -> bool {
        false
    }

    fn node_data(&self) -> &NodeData<CTX> {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData<CTX> {
        &mut self.node_data
    }

    fn update(&mut self, _self_node_ref: NodeRef<CTX>, _fgr_ctx: &mut CTX) -> bool {
        let result = self.value_changed;
        self.value_changed = false;
        result
    }

    fn dispose(&mut self, _ctx: &mut CTX) { /* do nothing */}
}

impl<CTX: HasFgrCtx + 'static, A: Send + Sync + 'static> Signal<CTX, A> {
    pub fn value<'a>(&'a self, ctx: &mut CTX) -> impl std::ops::Deref<Target=A> + 'a {
        if ctx.fgr_ctx().witness_observe {
            ctx.fgr_ctx().observed_nodes.push(self.into());
        }
        let impl_ = (*self.impl_).read().unwrap();
        struct MyRef<'a,CTX,A> {
            impl_: RwLockReadGuard<'a, SignalImpl<CTX,A>>,
        }
        impl<'a, CTX, A> std::ops::Deref for MyRef<'a, CTX, A> {
            type Target = A;
            fn deref(&self) -> &Self::Target {
                &self.impl_.value
            }
        }
        let val = MyRef { impl_: impl_ };
        val
    }

    pub fn update_value<CALLBACK: FnOnce(&mut A)>(&mut self, ctx: &mut CTX, callback: CALLBACK) {
        //
        if DEBUG_LOG {
            println!("Signal::update_value called on {:?}", (Into::<NodeRef<CTX>>::into(&*self)));
        }
        //
        ctx.fgr_batch(|ctx| {
            {
                let mut impl_ = (*self.impl_).write().unwrap();
                callback(&mut impl_.value);
                impl_.value_changed = true;
                impl_.node_data.flag = NodeFlag::Stale;
            }
            // add self to stack for propergating dependent flags to stale.
            ctx.fgr_ctx().stack.push((&*self).into());
            propergate_dependents_flags_to_stale(ctx);
        });
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum NodeFlag {
    Ready,
    Stale,
}

struct NodeData<CTX> {
    id: u64,
    flag: NodeFlag,
    changed: bool,
    dependencies: Vec<NodeRef<CTX>>,
    dependents: Vec<NodeRef<CTX>>,
    scoped: Vec<NodeRef<CTX>>,
}

trait IsNode<CTX: HasFgrCtx> {
    fn is_source(&self) -> bool;
    fn is_sink(&self) -> bool;
    fn node_data(&self) -> &NodeData<CTX>;
    fn node_data_mut(&mut self) -> &mut NodeData<CTX>;
    fn update(&mut self, self_node_ref: NodeRef<CTX>, ctx: &mut CTX) -> bool;
    fn dispose(&mut self, ctx: &mut CTX);
}

pub struct NodeRef<CTX> {
    id: u64,
    node: Arc<RwLock<dyn IsNode<CTX> + Send + Sync>>,
}

impl<CTX> std::fmt::Debug for NodeRef<CTX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Node Ref {})", self.id)
    }
}

impl<CTX> PartialEq for NodeRef<CTX> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.node, &other.node)
    }
}

impl<CTX> Clone for NodeRef<CTX> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            node: Arc::clone(&self.node),
        }
    }
}

impl<CTX> NodeRef<CTX> {
    fn with_node<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&dyn IsNode<CTX>) -> T,
    {
        f(&*(*self.node).read().unwrap())
    }
    fn with_node_mut<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut dyn IsNode<CTX>) -> T,
    {
        f(&mut *(*self.node).write().unwrap())
    }
}

fn update_graph<CTX: HasFgrCtx>(ctx: &mut CTX) {
    let mut tmp_buffer_1 = Vec::new();
    let mut tmp_buffer_2 = Vec::new();
    let mut stack = Vec::new();
    std::mem::swap(&mut ctx.fgr_ctx().tmp_buffer_1, &mut tmp_buffer_1);
    std::mem::swap(&mut ctx.fgr_ctx().tmp_buffer_2, &mut tmp_buffer_2);
    std::mem::swap(&mut ctx.fgr_ctx().stack, &mut stack);
    //
    if DEBUG_LOG {
        println!("update_graph: {} nodes", stack.len());
    }
    //
    loop {
        let Some(node) = stack.pop() else { break; };
        //
        if DEBUG_LOG {
            println!("at node {:?}", node);
        }
        //
        let flag = node.with_node(|n| n.node_data().flag);
        if DEBUG_LOG {
            println!("  flag: {:?}", flag);
        }
        match flag {
            NodeFlag::Ready => { /* do nothing */ },
            NodeFlag::Stale => {
                let is_source = node.with_node(|n| n.is_source());
                let is_sink = node.with_node(|n| n.is_sink());
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
                    if DEBUG_LOG {
                        println!("  update node {:?}", node);
                    }
                    let node2 = node.clone();
                    let changed = node.with_node_mut(|n| {
                        if !(is_source || is_sink) {
                            ctx.fgr_ctx().witness_created = true;
                            ctx.fgr_ctx().witness_observe = true;
                            for scoped in n.node_data_mut().scoped.drain(..) {
                                (*scoped.node).write().unwrap().dispose(ctx);
                            }
                        }
                        let changed = n.update(node2, ctx);
                        if !(is_source || is_sink) {
                            ctx.fgr_ctx().witness_observe = false;
                            ctx.fgr_ctx().witness_created = false;
                            let mut dependencies_to_remove: Vec<NodeRef<CTX>> = Vec::new();
                            let mut dependencies_to_add: Vec<NodeRef<CTX>> = Vec::new();
                            for dep in &ctx.fgr_ctx().observed_nodes {
                                let has = n.node_data().dependencies.contains(dep);
                                if !has {
                                    dependencies_to_add.push(dep.clone());
                                }
                            }
                            for dep in &n.node_data().dependencies {
                                let has = ctx.fgr_ctx().observed_nodes.contains(dep);
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
                            std::mem::swap(&mut n.node_data_mut().scoped, &mut ctx.fgr_ctx().created_nodes);
                        }
                        let n2 = n.node_data_mut();
                        n2.changed = changed;
                        if DEBUG_LOG {
                            println!("  changed = {}", changed);
                        }
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
    if DEBUG_LOG {
        println!("update_graph finished.");
    }
    //
    let mut deferred_effects = Vec::new();
    std::mem::swap(&mut ctx.fgr_ctx().defered_effects, &mut deferred_effects);
    for effect in deferred_effects {
        effect(ctx);
    }
}

fn propergate_dependents_flags_to_stale<CTX: HasFgrCtx + 'static>(ctx: &mut CTX) {
    loop {
        let Some(at) = ctx.fgr_ctx().stack.pop() else { break; };
        at.with_node(|n| {
            for dep in &n.node_data().dependents {
                ctx.fgr_ctx().tmp_buffer_1.push(dep.clone());
                ctx.fgr_ctx().tmp_buffer_2.push(dep.clone());
            }
        });
        let fgr_ctx = &mut *ctx.fgr_ctx();
        for dep in fgr_ctx.tmp_buffer_1.drain(..) {
            if DEBUG_LOG {
                println!("  dep: {:?} marked stale", dep);
            }
            dep.with_node_mut(|n| n.node_data_mut().flag = NodeFlag::Stale);
            fgr_ctx.stack.push(dep);
        }
    }
    let fgr_ctx = &mut *ctx.fgr_ctx();
    for dep in fgr_ctx.tmp_buffer_2.drain(..) {
        fgr_ctx.stack.push(dep);
    }
}

pub fn print_graph<CTX: HasFgrCtx>(node: NodeRef<CTX>) {
    println!("-- Graph Start --");
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        println!("  at node {:?}", node);
        node.with_node(|n| {
            println!("    dependencies: {:?}", n.node_data().dependencies);
            println!("    dependents: {:?}", n.node_data().dependents);
            println!("    scoped: {:?}", n.node_data().scoped);
            for dep in &n.node_data().dependents {
                stack.push(dep.clone());
            }
        });
    }
    println!("-- Graph End --");
}

#[macro_export]
macro_rules! cloned {
    (($($arg:ident),*) => $e:expr) => {{
        // clone all the args
        $(
            #[allow(unused_mut)]
            let mut $arg = ::std::clone::Clone::clone(&$arg);
        )*
        $e
    }};
}
