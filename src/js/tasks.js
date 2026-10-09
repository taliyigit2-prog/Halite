const waiters = new Map();
export function waitForTask(kind, taskId) {
  return new Promise((resolve) => waiters.set(`${kind}:${taskId}`, resolve));
}
export function settleTask(kind, payload, status) {
  const key = `${kind}:${payload.task_id}`;
  const resolve = waiters.get(key);
  if (!resolve) return;
  waiters.delete(key);
  resolve({ status, payload });
}
export function dropTaskWaiter(kind, taskId) { waiters.delete(`${kind}:${taskId}`); }
