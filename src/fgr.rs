use std::borrow::Borrow;
use std::rc::Rc;
use std::cell::{Ref, RefCell, RefMut};

pub struct FrgCtx {
    stack: Vec<NodeRef>,
    tmp_buffer_1: Vec<NodeRef>,
    tmp_buffer_2: Vec<NodeRef>,
}

pub struct Signal<A> {
    impl_: Rc<RefCell<SignalImpl<A>>>,
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
    fn node_data(&self) -> &NodeData {
        &self.node_data
    }

    fn node_data_mut(&mut self) -> &mut NodeData {
        &mut self.node_data
    }

    fn update(&mut self) -> bool {
        let result = self.value_changed;
        self.value_changed = false;
        result
    }
}

impl<A: 'static> Signal<A> {
    pub fn value<'a>(&'a self) -> Ref<'_, A> {
        let impl_ = (*self.impl_).borrow();
        let val = Ref::map(impl_, |impl_| &impl_.value);
        val
    }

    pub fn update_value<CALLBACK: FnOnce(&mut A)>(&mut self, fgr_ctx: &mut FrgCtx, callback: CALLBACK) {
        let mut impl_ = (*self.impl_).borrow_mut();
        callback(&mut impl_.value);
        impl_.value_changed = true;
        fgr_ctx.stack.push((&*self).into());
        propergate_dependents_flags_to_stale(fgr_ctx);
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum NodeFlag {
    Ready,
    Stale,
}

struct NodeData {
    flag: NodeFlag,
    changed: bool,
    visited: bool,
    dependencies: Vec<NodeRef>,
    dependents: Vec<NodeRef>,
}

trait IsNode {
    fn node_data(&self) -> &NodeData;
    fn node_data_mut(&mut self) -> &mut NodeData;
    fn update(&mut self) -> bool;
}

struct Node<UPDATE: FnMut() -> bool> {
    node: NodeData,
    update: UPDATE,
}

impl<UPDATE: FnMut() -> bool> IsNode for Node<UPDATE> {
    fn node_data(&self) -> &NodeData {
        &self.node
    }
    fn node_data_mut(&mut self) -> &mut NodeData {
        &mut self.node
    }
    fn update(&mut self) -> bool {
        (self.update)()
    }
}

pub struct NodeRef {
    node: Rc<RefCell<dyn IsNode>>,
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
    let tmp_buffer_1 = &mut fgr_ctx.tmp_buffer_1;
    let tmp_buffer_2 = &mut fgr_ctx.tmp_buffer_2;
    let stack = &mut fgr_ctx.stack;
    loop {
        let Some(node) = stack.pop() else { break; };
        let flag = node.with_node(|n| n.node_data().flag);
        match flag {
            NodeFlag::Ready => { /* do nothing */ },
            NodeFlag::Stale => {
                node.with_node(|n| {
                    for dep in &n.node_data().dependencies {
                        tmp_buffer_2.push(dep.clone());
                    }
                });
                let mut any_depdencies_changed = false;
                let mut has_stale_dependencies = false;
                for dep in &*tmp_buffer_2 {
                    let flag = dep.with_node(|n| n.node_data().flag);
                    match flag {
                        NodeFlag::Ready => {
                            let changed = dep.with_node(|n| n.node_data().changed);
                            if changed {
                                any_depdencies_changed = true;
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
                if !has_stale_dependencies && any_depdencies_changed {
                    let changed = node.with_node_mut(|n| {
                        let changed = n.update();
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
}

fn propergate_dependents_flags_to_stale(fgr_ctx: &mut FrgCtx) {
    loop {
        let Some(at) = fgr_ctx.stack.pop() else { break; };
        at.with_node(|n| {
            for dep in &n.node_data().dependents {
                fgr_ctx.tmp_buffer_1.push(dep.clone());
            }
        });
        for dep in fgr_ctx.tmp_buffer_1.drain(..) {
            dep.with_node_mut(|n| n.node_data_mut().flag = NodeFlag::Stale);
            fgr_ctx.stack.push(dep);
        }
    }
}
