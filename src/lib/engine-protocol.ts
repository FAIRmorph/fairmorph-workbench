import { z } from 'zod';

const ProgressPayloadSchema = z.object({
  event: z.literal('progress'),
  stage: z.string().min(1),
  percentage: z.number().finite().min(0).max(100),
  message: z.string()
});

const ResultDataSchema = z.object({
  processed_records: z.number().int().nonnegative(),
  engine_python_version: z.string().min(1),
  dataset_sha256: z.string().regex(/^[0-9a-fA-F]{64}$/),
  completed_at: z.string().datetime({ offset: true }),
  loaded_file: z.string().min(1)
});

const SuccessPayloadSchema = z.object({
  event: z.literal('result'),
  status: z.literal('success'),
  data: ResultDataSchema
});

const ErrorPayloadSchema = z.object({
  event: z.literal('result'),
  status: z.literal('error'),
  message: z.string().min(1)
});

export const EngineEventSchema = z.union([
  ProgressPayloadSchema,
  SuccessPayloadSchema,
  ErrorPayloadSchema
]);

export type EngineEvent = z.infer<typeof EngineEventSchema>;
export type EngineResultData = z.infer<typeof ResultDataSchema>;

export function parseEngineEvent(raw: string): EngineEvent {
  return EngineEventSchema.parse(JSON.parse(raw));
}
