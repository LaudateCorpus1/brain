use parser::{Statement, Slice, Expression};
use instructions::Instructions;
use memory::MemoryLayout;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    // Illegal redeclaration of a name
    IllegalRedeclaration {
        name: String,
    },
    // Name used before it was declared
    UndeclaredIdentifier {
        name: String,
    },
    // Tried to declare a zero size variable
    DeclaredZeroSize {
        name: String,
    },
    // Declaration contained a size, but it was invalid
    DeclaredIncorrectSize {
        name: String,
        expected: usize,
        actual: usize,
    },
}

/// Expands the given statement into instructions
pub fn expand(
    instructions: &mut Instructions,
    mem: &mut MemoryLayout,
    stmt: Statement
) -> Result<(), Error> {
    match stmt {
        Statement::Comment(_) => Ok(()),
        Statement::Output(expr) => output_expr(instructions, mem, expr),
        Statement::Declaration {name, slice, expr} => declare(instructions, mem, name, slice, expr),
    }
}

fn output_expr(
    instructions: &mut Instructions,
    mem: &mut MemoryLayout,
    expr: Expression
) -> Result<(), Error> {
    match expr {
        Expression::StringLiteral(text) => {
            let cell = mem.next_available_cell();
            let start_cell = mem.current_cell();

            instructions.move_relative(start_cell, cell);
            write_string_literal(instructions, text.as_bytes());
            instructions.move_relative(cell, start_cell);

            Ok(())
        },
        Expression::Identifier(ident) => {
            let (position, size) = mem.get_cell_contents(&ident).ok_or_else(|| {
                Error::UndeclaredIdentifier {name: ident}
            })?;

            instructions.move_relative(mem.current_cell(), position);
            instructions.write_consecutive(size);
            // This way we end up one cell after the last written one
            instructions.move_right();
            mem.set_current_cell(position + size);

            Ok(())
        },
    }
}

fn write_string_literal(instructions: &mut Instructions, bytes: &[u8]) {
    // Writing string literals are special because you don't necessarily
    // need to store the string literal in any location outside of what is necessary
    // for the write. The memory is to be allocated briefly, then freed.
    // Because of this, we don't split allocation and writing into separate steps.
    // We keep this special routine specifically designed to write string literals

    let mut last_char: u8 = 0;
    for ch in bytes {
        let ch = *ch;
        instructions.increment_relative(last_char, ch);
        instructions.write();

        last_char = ch;
    }

    // always reset this cell because we don't need it anymore
    instructions.increment_relative(last_char, 0);
}

fn declare(
    instructions: &mut Instructions,
    mem: &mut MemoryLayout,
    name: String,
    slice: Option<Slice>,
    expr: Expression
) -> Result<(), Error> {
    if mem.is_declared(&name) {
        if slice.is_some() {
            return Err(Error::IllegalRedeclaration {name: name});
        }

        unimplemented!();
    }

    // Name is not declared
    else {
        declare_undeclared(instructions, mem, name, slice, expr)
    }
}

/// Declares a new identifier that was previously undeclared
fn declare_undeclared(
    instructions: &mut Instructions,
    mem: &mut MemoryLayout,
    name: String,
    slice: Option<Slice>,
    expr: Expression
) -> Result<(), Error> {
    if slice.is_none() {
        return Err(Error::UndeclaredIdentifier {name: name});
    }
    let slice = slice.unwrap();

    match expr {
        Expression::StringLiteral(value) => {
            let size = match slice {
                Slice::SingleValue(s) => s,
                Slice::Unspecified => value.len(),
            };

            if size == 0 {
                return Err(Error::DeclaredZeroSize {
                    name: name,
                });
            }
            else if size != value.len() {
                return Err(Error::DeclaredIncorrectSize {
                    name: name,
                    expected: value.len(),
                    actual: size,
                });
            }

            let position = mem.declare(&name, size);
            instructions.move_relative(mem.current_cell(), position);
            instructions.store_bytes(value.as_bytes());
            mem.set_current_cell(position + value.len());
            Ok(())
        },
        Expression::Identifier(value_name) => {
            let (source_position, source_size) = mem.get_cell_contents(&value_name).ok_or_else(|| Error::UndeclaredIdentifier {name: value_name})?;

            let size = match slice {
                Slice::SingleValue(s) => s,
                Slice::Unspecified => source_size,
            };

            if size != source_size {
                return Err(Error::DeclaredIncorrectSize {
                    name: name,
                    expected: source_size,
                    actual: size,
                });
            }

            let position = mem.declare(&name, size);
            copy_cells(instructions, mem, source_position, position, size);
            Ok(())
        }
    }
}

/// Generates brainfuck instructions to copy `size` cells from
/// the source position to the target position
fn copy_cells(
    instructions: &mut Instructions,
    mem: &mut MemoryLayout,
    source: usize,
    target: usize,
    size: usize
) {
    let start = mem.current_cell();
    // We need a hold cell to temporarily hold the value of the source
    // while we move it to the target
    // Once that initial move is done, we move the value of the hold cell back
    // to the source
    // These two moves with a temporary cell simulate a copy in brainfuck
    let hold = mem.next_available_cell();

    // Since size can be more than u8, we need to generate instructions for every cell
    // in a loop like this. We can't just store size in a cell and then use it to do these
    // instructions in a loop
    for i in 0..size {
        instructions.move_relative(start, source + i);

        instructions.jump_forward_if_zero();
        instructions.decrement();

        //TODO: This could be a source of optimization since we're potentially
        //TODO: doing extra movement instructions we don't need to
        instructions.move_relative(source + i, hold);
        instructions.increment();

        instructions.move_relative(hold, target + i);
        instructions.increment();

        instructions.move_relative(target + i, source + i);
        instructions.jump_backward_unless_zero();

        // Move from hold back to source leaving everything as it was with
        // source copied into target
        // hold is zero again at the end of this process
        instructions.move_relative(source + i, hold);
        instructions.jump_forward_if_zero();

        instructions.decrement();
        instructions.move_relative(hold, source + i);
        instructions.increment();
        instructions.move_relative(source + i, hold);

        instructions.jump_backward_unless_zero();

        // Return to the starting position
        instructions.move_relative(hold, start);
    }
}
