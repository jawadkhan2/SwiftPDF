// Last page the user was looking at in the Viewer. Other tools (e.g. Fill &
// Sign) read this so opening them lands on the same page the user just had open.
// Reset to 0 whenever a fresh document is loaded.

export const viewer = $state<{ page: number }>({ page: 0 });
