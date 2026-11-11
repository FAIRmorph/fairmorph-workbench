/** Pure setup-state rules shared by Svelte and Vitest. No scientific inference. */
export type SourceRole = '' | 'events' | 'counts';
export type SourceFile = {
  path: string;
  role: SourceRole;
  imported: boolean;
  checksum: string;
  resourceId: string;
};
export type PipelineInput = { name: string; format: 'csv' | 'parquet' };

export function sourceFileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function canInitializeProject(name: string, parent: string, folder: string): boolean {
  const leaf = folder.trim();
  return !!name.trim() && !!parent.trim() && !!leaf
    && leaf !== '.' && leaf !== '..'
    && !/[\\/]/.test(leaf);
}

export function importsReady(files: SourceFile[]): boolean {
  const pending = files.filter(file => !file.imported);
  return pending.length > 0 && pending.every(file => file.role === 'events' || file.role === 'counts');
}

export function pipelineBindingsComplete(
  inputs: PipelineInput[] | undefined,
  files: SourceFile[],
  bindings: Record<string, string>,
  formats: Record<string, 'csv' | 'parquet'>
): boolean {
  return !!inputs?.length && inputs.every(input => {
    const id = bindings[input.name];
    const resource = files.find(file => file.resourceId === id && file.imported);
    return !!resource && (formats[input.name] === 'csv' || formats[input.name] === 'parquet');
  });
}

/** UI gate only. The Engine separately validates RUN eligibility at execution. */
export function runButtonReady(
  currentProject: string,
  setupProject: string,
  selectedPipelineSha: string
): boolean {
  return !!setupProject
    && currentProject.trim() === setupProject
    && /^[0-9a-f]{64}$/.test(selectedPipelineSha);
}
