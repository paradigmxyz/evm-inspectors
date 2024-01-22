use alloy_primitives::{Log, U256};
use revm::{
    interpreter::{
        CallInputs, CallOutcome, CreateInputs, CreateOutcome, Interpreter, InterpreterResult,
    },
    primitives::{db::Database, Address},
    EvmContext, Inspector,
};
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

/// An [Inspector] that is either owned by an individual [Inspector] or is shared as part of a
/// series of inspectors in a [InspectorStack](crate::stack::InspectorStack).
///
/// Caution: if the [Inspector] is _stacked_ then it _must_ be called first.
#[derive(Debug)]
pub enum MaybeOwnedInspector<INSP> {
    /// Inspector is owned.
    Owned(Rc<RefCell<INSP>>),
    /// Inspector is shared and part of a stack
    Stacked(Rc<RefCell<INSP>>),
}

impl<INSP> MaybeOwnedInspector<INSP> {
    /// Create a new _owned_ instance
    pub fn new_owned(inspector: INSP) -> Self {
        MaybeOwnedInspector::Owned(Rc::new(RefCell::new(inspector)))
    }

    /// Creates a [MaybeOwnedInspector::Stacked] clone of this type.
    pub fn clone_stacked(&self) -> Self {
        match self {
            MaybeOwnedInspector::Owned(gas) | MaybeOwnedInspector::Stacked(gas) => {
                MaybeOwnedInspector::Stacked(Rc::clone(gas))
            }
        }
    }

    /// Returns a reference to the inspector.
    pub fn as_ref(&self) -> Ref<'_, INSP> {
        match self {
            MaybeOwnedInspector::Owned(insp) => insp.borrow(),
            MaybeOwnedInspector::Stacked(insp) => insp.borrow(),
        }
    }
}

impl<INSP: Default> MaybeOwnedInspector<INSP> {
    /// Create a new _owned_ instance
    pub fn owned() -> Self {
        Self::new_owned(Default::default())
    }
}

impl<INSP: Default> Default for MaybeOwnedInspector<INSP> {
    fn default() -> Self {
        Self::owned()
    }
}

impl<INSP> Clone for MaybeOwnedInspector<INSP> {
    fn clone(&self) -> Self {
        self.clone_stacked()
    }
}

impl<INSP, DB> Inspector<DB> for MaybeOwnedInspector<INSP>
where
    DB: Database,
    INSP: Inspector<DB>,
{
    fn initialize_interp(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        match self {
            MaybeOwnedInspector::Owned(insp) => {
                insp.borrow_mut().initialize_interp(interp, context)
            }
            MaybeOwnedInspector::Stacked(_) => {}
        }
    }

    fn step(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        match self {
            MaybeOwnedInspector::Owned(insp) => insp.borrow_mut().step(interp, context),
            MaybeOwnedInspector::Stacked(_) => {}
        }
    }

    fn log(&mut self, context: &mut EvmContext<DB>, log: &Log) {
        match self {
            MaybeOwnedInspector::Owned(insp) => return insp.borrow_mut().log(context, log),
            MaybeOwnedInspector::Stacked(_) => {}
        }
    }

    fn step_end(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        match self {
            MaybeOwnedInspector::Owned(insp) => insp.borrow_mut().step_end(interp, context),
            MaybeOwnedInspector::Stacked(_) => {}
        }
    }

    fn call(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &mut CallInputs,
    ) -> Option<CallOutcome> {
        match self {
            MaybeOwnedInspector::Owned(insp) => return insp.borrow_mut().call(context, inputs),
            MaybeOwnedInspector::Stacked(_) => None,
        }
    }

    fn call_end(
        &mut self,
        context: &mut EvmContext<DB>,
        result: InterpreterResult,
    ) -> InterpreterResult {
        match self {
            MaybeOwnedInspector::Owned(insp) => insp.borrow_mut().call_end(context, result),
            MaybeOwnedInspector::Stacked(_) => result,
        }
    }

    fn create(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &mut CreateInputs,
    ) -> Option<CreateOutcome> {
        match self {
            MaybeOwnedInspector::Owned(insp) => return insp.borrow_mut().create(context, inputs),
            MaybeOwnedInspector::Stacked(_) => None,
        }
    }

    fn create_end(
        &mut self,
        context: &mut EvmContext<DB>,
        result: InterpreterResult,
        address: Option<Address>,
    ) -> CreateOutcome {
        match self {
            MaybeOwnedInspector::Owned(insp) => {
                return insp.borrow_mut().create_end(context, result, address)
            }
            MaybeOwnedInspector::Stacked(_) => CreateOutcome::new(result, address),
        }
    }

    fn selfdestruct(&mut self, contract: Address, target: Address, value: U256) {
        match self {
            MaybeOwnedInspector::Owned(insp) => {
                return insp.borrow_mut().selfdestruct(contract, target, value)
            }
            MaybeOwnedInspector::Stacked(_) => {}
        }
    }
}
