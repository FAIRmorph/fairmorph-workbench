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
