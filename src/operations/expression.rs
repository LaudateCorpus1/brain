use parser::Expression;
use parser::Expression::*;
use memory::MemoryBlock;

use super::OperationsResult;
use super::scope::{TypeId, ScopeStack};

/// Generates operations for evaluating the given expression
/// and storing its result in the given target memory block
pub fn into_operations(
    scope: &mut ScopeStack,
    expr: Expression,
    target_type: TypeId,
    target: MemoryBlock,
) -> OperationsResult {
    match expr {
        //Call {method, args} =>
        _ => unimplemented!(),
    }
}
