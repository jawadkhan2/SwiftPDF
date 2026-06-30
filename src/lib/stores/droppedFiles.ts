let pendingMergePaths: string[] = [];

export function queueMergeDrop(paths: string[]) {
  pendingMergePaths = [...paths];
}

export function takeMergeDrop(): string[] {
  const paths = pendingMergePaths;
  pendingMergePaths = [];
  return paths;
}
