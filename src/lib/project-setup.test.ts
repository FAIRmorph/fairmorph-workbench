import { describe, expect, it } from 'vitest';
import {
  canInitializeProject, importsReady, pipelineBindingsComplete,
  runButtonReady, sourceFileName, type SourceFile
} from './project-setup';

const raw: SourceFile = {
  path: '/tmp/raw/events.csv', role: '', imported: false, checksum: '', resourceId: ''
};
const importedEvents: SourceFile = {
  ...raw, role: 'events', imported: true, checksum: 'a'.repeat(64),
  resourceId: 'resource-events-aaa'
};
const importedCounts: SourceFile = {
  ...raw, path: '/tmp/raw/counts.csv', role: 'counts', imported: true,
  checksum: 'b'.repeat(64), resourceId: 'resource-counts-bbb'
};

describe('Svelte project setup readiness', () => {
  it('requires explicit project name, parent, and a safe new leaf', () => {
    expect(canInitializeProject(' Survey ', '/tmp', 'survey.fairmorph')).toBe(true);
    for (const folder of ['', '.', '..', '../outside', 'other/nested', 'other\\nested']) {
      expect(canInitializeProject('Survey', '/tmp', folder)).toBe(false);
    }
    expect(canInitializeProject('', '/tmp', 'survey.fairmorph')).toBe(false);
    expect(canInitializeProject('Survey', '', 'survey.fairmorph')).toBe(false);
  });

  it('never guesses source roles and requires all pending sources to be explicitly typed', () => {
    expect(importsReady([])).toBe(false);
    expect(importsReady([raw])).toBe(false);
    expect(importsReady([{ ...raw, role: 'events' }])).toBe(true);
    expect(importsReady([{ ...raw, role: 'counts' }, raw])).toBe(false);
    expect(importsReady([importedEvents, raw])).toBe(false);
    expect(importsReady([importedEvents, importedCounts])).toBe(false);
  });

  it('supports retrying the non-imported remainder after a partial failure', () => {
    const remaining: SourceFile = { ...raw, path: '/tmp/raw/counts.csv', role: 'counts' };
    expect(importsReady([importedEvents, remaining])).toBe(true);
    expect(importsReady([importedEvents, { ...remaining, role: '' }])).toBe(false);
  });

  it('requires every pipeline slot to match an actually imported resource', () => {
    const inputs = [
      { name: 'observations', format: 'csv' as const },
      { name: 'visits', format: 'csv' as const }
    ];
    const formats = { observations: 'csv' as const, visits: 'csv' as const };
    const selected = {
      observations: importedCounts.resourceId,
      visits: importedEvents.resourceId
    };
    expect(pipelineBindingsComplete(inputs, [importedEvents, importedCounts], selected, formats)).toBe(true);
    expect(pipelineBindingsComplete(inputs, [importedEvents], selected, formats)).toBe(false);
    expect(pipelineBindingsComplete(inputs, [importedEvents, importedCounts],
      { observations: selected.observations, visits: '' }, formats)).toBe(false);
    expect(pipelineBindingsComplete(inputs, [importedEvents, importedCounts],
      selected, { observations: 'csv' })).toBe(false);
    expect(pipelineBindingsComplete([], [importedEvents], selected, formats)).toBe(false);
    expect(pipelineBindingsComplete(undefined, [importedEvents], selected, formats)).toBe(false);
  });

  it('keeps RUN disabled until the current project has an explicitly selected pipeline', () => {
    const project = '/tmp/study.fairmorph';
    const sha = 'a'.repeat(64);
    expect(runButtonReady(project, project, '')).toBe(false);
    expect(runButtonReady(project, project, 'invalid')).toBe(false);
    expect(runButtonReady(project, project, sha)).toBe(true);
    expect(runButtonReady('/tmp/another.fairmorph', project, sha)).toBe(false);
    expect(runButtonReady(project, '', sha)).toBe(false);
    expect(runButtonReady(project, project, 'A'.repeat(64))).toBe(false);
  });

  it('extracts filenames from Windows and POSIX paths without touching input bytes', () => {
    expect(sourceFileName('C:\\surveys\\counts.csv')).toBe('counts.csv');
    expect(sourceFileName('/mnt/surveys/events.csv')).toBe('events.csv');
  });
});
