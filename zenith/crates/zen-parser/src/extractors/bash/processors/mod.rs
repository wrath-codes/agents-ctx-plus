mod commands;
mod control_flow;
mod core;
mod declarations;
mod structures;

pub(super) use commands::process_command;
pub(super) use control_flow::{
    process_c_style_for, process_case_statement, process_for_statement, process_if_statement,
    process_while_statement,
};
pub(super) use core::{process_function, process_shebang};
pub(super) use declarations::{process_declaration_command, process_variable_assignment};
pub(super) use structures::{
    process_command_group, process_list, process_negated_command, process_pipeline,
    process_redirected_statement, process_subshell, process_test_command, process_unset_command,
};
