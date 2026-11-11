import { z } from 'zod';

// The Engine owns this protocol. The Workbench validates only the event envelope
// and known result fields; it never interprets scientific results.
export const ProgressEvent = z.object({
  event: z.literal('progress'),
  stage: z.string().min(1),
  percentage: z.number().finite().min(0).max(100),
  message: z.string()
});
export const RunOutput = z.object({
  name: z.string().min(1),
  rows: z.number().int().nonnegative(),
  columns: z.array(z.string())
});
export const RunResult = z.object({
  run_id: z.string().min(1),
  outputs: z.array(RunOutput)
});
export const HandshakeResult = z.object({
  protocol_version: z.literal('fairmorph.poc.ndjson/1'),
  run_schema_version: z.literal('fairmorph.poc.run/2'),
  engine_version: z.string().min(1),
  commands: z.array(z.string())
});
export const DatasetMetadataStatus = z.union([
  z.object({ status: z.literal('unconfigured'), revision_sha256: z.null(), title: z.null() }),
  z.object({
    status: z.literal('approved'),
    revision_sha256: z.string().regex(/^[0-9a-f]{64}$/),
    title: z.string().min(5),
    confirmed_by: z.string().min(1),
    confirmed_at: z.string().min(1)
  })
]);
export const StagedDatasetMetadata = z.object({
  status: z.literal('staged'),
  revision_sha256: z.string().regex(/^[0-9a-f]{64}$/),
  title: z.string().min(5)
});
export const ApprovedDatasetMetadata = z.object({
  status: z.literal('approved'),
  revision_sha256: z.string().regex(/^[0-9a-f]{64}$/),
  title: z.string().min(5),
  confirmed_by: z.string().min(1),
  confirmed_at: z.string().min(1)
});
export type DatasetMetadata = z.infer<typeof DatasetMetadataStatus>;

export const ErrorEvent = z.object({
  event: z.literal('result'),
  status: z.literal('error'),
  message: z.string().min(1)
});
export const EngineEvent = z.union([
  ProgressEvent,
  z.object({ event: z.literal('result'), status: z.literal('success'), data: z.unknown() }),
  ErrorEvent
]);

export type Progress = z.infer<typeof ProgressEvent>;
export type RunSummary = z.infer<typeof RunResult>;

export function parseEngineEvent(raw: unknown) {
  return EngineEvent.parse(typeof raw === 'string' ? JSON.parse(raw) : raw);
}

export const CreatedProject = z.object({
  project_id: z.string().uuid(),
  project_path: z.string().min(1)
});
export const ImportedResource = z.object({
  resource_id: z.string().min(1),
  role: z.enum(['events', 'counts']),
  original_name: z.string().min(1),
  sha256: z.string().regex(/^[0-9a-f]{64}$/)
});
export type ImportedSource = z.infer<typeof ImportedResource>;

export const PipelineTemplate = z.object({
  path: z.string().min(1),
  inputs: z.array(z.object({
    name: z.string().min(1),
    format: z.enum(['csv', 'parquet'])
  })).min(1),
  outputs: z.array(z.string().min(1)).min(1),
  steps: z.number().int().positive()
});
export const SavedPipelineDraft = z.object({
  path: z.string().min(1),
  inputs: z.number().int().positive()
});
export const RegisteredPipeline = z.object({
  pipeline_sha256: z.string().regex(/^[0-9a-f]{64}$/)
});
export const SelectedPipeline = z.object({
  project_id: z.string().uuid(),
  active_pipeline_sha256: z.string().regex(/^[0-9a-f]{64}$/)
});
export type PipelineTemplateInfo = z.infer<typeof PipelineTemplate>;

/** Engine verify-project result; integrity verification is not scientific validation. */
export const ProjectVerification = z.object({
  project_path: z.string().min(1),
  source_objects: z.number().int().nonnegative(),
  completed_runs: z.number().int().nonnegative(),
  exports: z.number().int().nonnegative(),
  status: z.literal('verified')
});
export type ProjectVerificationResult = z.infer<typeof ProjectVerification>;
