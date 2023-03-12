//! ... conceptually infinite ...

use super::operation::Operation;
use smallvec::SmallVec;
use std::{cell::RefCell, collections::HashMap, marker::PhantomData};

thread_local! {
    static EXPRS: RefCell<Vec<ExprData>> = RefCell::new(Vec::new());
    static LOOKUP: RefCell<HashMap<ExprData, ExprId>> = RefCell::new(HashMap::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId(
    usize,
    /// As an index into a thread-local vector, it is a logic error to send this to a different thread.
    PhantomData<*mut ()>,
);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExprData {
    pub operation: Operation,
    pub children: SmallVec<[ExprId; 2]>,
}

pub fn expr_data(id: ExprId) -> ExprData {
    EXPRS.with(|exprs| exprs.borrow()[id.0].clone())
}

pub fn expr_id(data: &ExprData) -> ExprId {
    LOOKUP.with(|lookup| {
        *lookup.borrow_mut().entry(data.clone()).or_insert_with(|| {
            EXPRS.with(|exprs| {
                let mut exprs = exprs.borrow_mut();
                let id = ExprId(exprs.len(), PhantomData);
                exprs.push(data.clone());
                id
            })
        })
    })
}
