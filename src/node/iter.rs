#[cfg(feature = "range")] use compare::Compare;
#[cfg(feature = "range")] use std::cmp::Ordering::*;
#[cfg(feature = "range")] use std::collections::Bound;
use std::collections::VecDeque;
use super::Node;

pub trait NodeRef {
    type Key;
    type Item;
    fn key(&self) -> &Self::Key;
    fn item(self) -> Self::Item;
    fn left(&mut self) -> Option<Self>;
    fn right(&mut self) -> Option<Self>;
}

pub struct MarkedNode<'a, K: 'a, V: 'a> {
    node: &'a Node<K, V>,
    seen_l: bool,
    seen_r: bool,
}

impl<'a, K, V> Clone for MarkedNode<'a, K, V> {
    fn clone(&self) -> Self { *self }
}

impl<'a, K, V> Copy for MarkedNode<'a, K, V> {}

impl<'a, K, V> MarkedNode<'a, K, V> {
    pub fn new(node: &'a Box<Node<K, V>>) -> Self {
        MarkedNode { node: node, seen_l: false, seen_r: false }
    }
}

impl<'a, K, V> NodeRef for MarkedNode<'a, K, V> {
    type Key = K;
    type Item = (&'a K, &'a V);
    fn key(&self) -> &K { &self.node.key }
    fn item(self) -> (&'a K, &'a V) { (&self.node.key, &self.node.value) }

    fn left(&mut self) -> Option<Self> {
        if self.seen_l {
            None
        } else {
            self.seen_l = true;
            self.node.left.as_ref().map(MarkedNode::new)
        }
    }

    fn right(&mut self) -> Option<Self> {
        if self.seen_r {
            None
        } else {
            self.seen_r = true;
            self.node.right.as_ref().map(MarkedNode::new)
        }
    }
}

impl<K, V> NodeRef for Box<Node<K, V>> {
    type Key = K;
    type Item = (K, V);
    fn key(&self) -> &K { &self.key }
    fn item(self) -> (K, V) { let node = *self; (node.key, node.value) }
    fn left(&mut self) -> Option<Self> { self.left.take() }
    fn right(&mut self) -> Option<Self> { self.right.take() }
}

#[derive(Clone)]
pub struct Iter<N> where N: NodeRef {
    nodes: VecDeque<N>,
    size: usize,
}

macro_rules! bound {
    ($iter:expr,
     $cmp:expr,
     $bound:expr,
     $ordering_pre:ident,
     $ordering_post:ident,
     $pre:ident,
     $post:ident,
     $mut_:ident,
     $pop:ident,
     $push:ident
    ) => {
        if let Some((key, inc)) = bound_to_opt($bound) {
            loop {
                let op = match $iter.nodes.$mut_() {
                    None => break,
                    Some(node) => match $cmp.compare(key, node.key()) {
                        Equal =>
                            if inc {
                                if node.$pre().is_some() { $iter.size -= 1; }
                                break;
                            } else {
                                Op::PopPush(node.$post(), true)
                            },
                        $ordering_post => Op::PopPush(node.$post(), false),
                        $ordering_pre => Op::Push(node.$pre()),
                    },
                };

                match op {
                    Op::Push(node_ref) => match node_ref {
                        None => break,
                        Some(node) => $iter.nodes.$push(node),
                    },
                    Op::PopPush(node_ref, terminate) => {
                        $iter.nodes.$pop();
                        $iter.size -= 1;
                        if let Some(node) = node_ref { $iter.nodes.$push(node); }
                        if terminate { break; }
                    }
                }
            }
        }
    }
}

impl<N> Iter<N> where N: NodeRef {
    pub fn new(root: Option<N>, size: usize) -> Self {
        Iter { nodes: root.into_iter().collect(), size: size }
    }

    #[cfg(feature = "range")]
    pub fn range<C, Min: ?Sized, Max: ?Sized>(root: Option<N>, size: usize, cmp: &C,
                                              min: Bound<&Min>, max: Bound<&Max>)
        -> Self where C: Compare<Min, N::Key> + Compare<Max, N::Key> {

        fn bound_to_opt<T>(bound: Bound<T>) -> Option<(T, bool)> {
            match bound {
                Bound::Unbounded => None,
                Bound::Included(bound) => Some((bound, true)),
                Bound::Excluded(bound) => Some((bound, false)),
            }
        }

        enum Op<T> {
            PopPush(Option<T>, bool),
            Push(Option<T>),
        }

        let mut it = Iter::new(root, size);

        bound!(it, cmp, min, Less, Greater, left, right, back_mut, pop_back, push_back);
        bound!(it, cmp, max, Greater, Less, right, left, front_mut, pop_front, push_front);

        it
    }

    #[cfg(feature = "range")]
    pub fn range_size_hint(&self) -> (usize, Option<usize>) {
        (self.nodes.len(), Some(self.size))
    }
}

impl<N> Iterator for Iter<N> where N: NodeRef {
    type Item = N::Item;

    fn next(&mut self) -> Option<N::Item> {
        loop {
            let push = self.nodes.back_mut().and_then(N::left);

            match push {
                None => return self.nodes.pop_back().map(|mut node| {
                    self.size -= 1;
                    if let Some(right) = node.right() { self.nodes.push_back(right); }
                    node.item()
                }),
                Some(left) => self.nodes.push_back(left),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (self.size, Some(self.size)) }
}

impl<N> DoubleEndedIterator for Iter<N> where N: NodeRef {
    fn next_back(&mut self) -> Option<N::Item> {
        loop {
            let push = self.nodes.front_mut().and_then(N::right);

            match push {
                None => return self.nodes.pop_front().map(|mut node| {
                    self.size -= 1;
                    if let Some(left) = node.left() { self.nodes.push_front(left); }
                    node.item()
                }),
                Some(right) => self.nodes.push_front(right),
            }
        }
    }
}
