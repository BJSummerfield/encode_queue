//! Application/service layer for the queue.
//!
//! Scaffold (WP01): `QueueService` is a shell that holds the three injected
//! ports; its `add`/`ls`/`start`/`edit` methods (and the `QueueError` that
//! composes the boundary errors via `#[from]`) are implemented in WP08. It
//! already establishes the intended dependency direction: the service depends
//! only on `crate::domain` and `crate::ports` (ARCH-01).
//!
//! The fields and `new` are unused until WP08 wires the operations; the
//! resulting `dead_code` warnings are expected scaffold-stage noise (the
//! `-D warnings` gate is a WP10 property).

use crate::ports::command_runner::CommandRunner;
use crate::ports::editor::EditorLauncher;
use crate::ports::queue_store::QueueStore;

/// The queue service, generic over its three injected ports.
///
/// A third port generic (`E: EditorLauncher`) beyond the `QueueService<R, C>`
/// shorthand is required so that all three ports can be injected with mocks
/// (TST-01).
#[derive(Debug, Clone)]
pub struct QueueService<R, C, E>
where
    R: QueueStore,
    C: CommandRunner,
    E: EditorLauncher,
{
    store: R,
    runner: C,
    editor: E,
}

impl<R, C, E> QueueService<R, C, E>
where
    R: QueueStore,
    C: CommandRunner,
    E: EditorLauncher,
{
    /// Wire the three ports into a service.
    pub fn new(store: R, runner: C, editor: E) -> Self {
        Self {
            store,
            runner,
            editor,
        }
    }
}
