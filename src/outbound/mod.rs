//! Outbound adapters — domain newtypes → external I/O: file adapters
//! (queue + failure logs), shell runner, editor, and queue lock.

pub mod editor;
pub mod lock;
pub mod queue_file;
pub mod shell;
